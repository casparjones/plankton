// Handler für Task-CRUD und Task-Move.

use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{Local, Utc};
use uuid::Uuid;

use crate::error::ApiError;
use crate::models::*;
use crate::services::{extract_token_from_headers, publish_event, publish_update, validate_jwt};
use crate::state::AppState;

/// Caller-Identität aus Headers auflösen (JWT oder Agent-Token).
async fn resolve_caller(headers: &axum::http::HeaderMap, state: &AppState) -> String {
    if let Some(t) = extract_token_from_headers(headers) {
        if let Ok(claims) = validate_jwt(&t, &state.jwt_secret) {
            return claims.display_name;
        }
    }
    if let Some(bearer) = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
    {
        if let Ok(agent_token) = state.store.get_token_by_value(bearer).await {
            return agent_token.name;
        }
    }
    "anonymous".to_string()
}

/// POST /api/projects/:id/import – Mehrere Tasks auf einmal importieren.
pub async fn import_tasks(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: axum::http::HeaderMap,
    Json(req): Json<ImportRequest>,
) -> Result<Json<ImportResponse>, ApiError> {
    let mut project = state.store.resolve_project(&id).await?;
    let user_name = resolve_caller(&headers, &state).await;

    let now = Utc::now().to_rfc3339();
    let today = Local::now().format("%Y-%m-%d %H:%M").to_string();

    // Find default column (TODO slug or first non-hidden)
    let default_col_id = project.columns.iter()
        .find(|c| c.slug == "TODO")
        .or_else(|| project.columns.iter().find(|c| !c.hidden))
        .map(|c| c.id.clone())
        .unwrap_or_default();

    let mut imported = 0;
    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    let mut skipped = 0;

    for (i, mut task) in req.tasks.into_iter().enumerate() {
        let idx = i + 1;

        // Validate title (required)
        if task.title.trim().is_empty() {
            errors.push(format!("Task #{}: title is required", idx));
            skipped += 1;
            continue;
        }

        // Validate points range
        if task.points < 0 || task.points > 100 {
            errors.push(format!("Task #{} \"{}\": points must be 0-100, got {}", idx, task.title, task.points));
            skipped += 1;
            continue;
        }

        // Resolve column_slug to column_id
        if task.column_id.is_empty() && !task.column_slug.is_empty() {
            let slug = task.column_slug.to_uppercase();
            if let Some(col) = project.columns.iter().find(|c| c.slug == slug) {
                task.column_id = col.id.clone();
            } else {
                // Fallback auf TODO statt Fehler
                task.column_id = default_col_id.clone();
                warnings.push(format!("Task #{} \"{}\": column_slug '{}' unbekannt, verwende TODO", idx, task.title, task.column_slug));
            }
        }
        task.column_slug.clear();

        // Unbekannte column_id → Fallback auf TODO
        if !task.column_id.is_empty() && !project.columns.iter().any(|c| c.id == task.column_id) {
            warnings.push(format!("Task #{} \"{}\": column_id '{}' unbekannt, verwende TODO", idx, task.title, task.column_id));
            task.column_id = default_col_id.clone();
        }

        // Keine Spalte angegeben → Fallback auf TODO
        if task.column_id.is_empty() {
            task.column_id = default_col_id.clone();
            warnings.push(format!("Task #{} \"{}\": keine Spalte angegeben, verwende TODO", idx, task.title));
        }

        // Auto-set fields
        task.id = Uuid::new_v4().to_string();
        task.slug = unique_task_slug(&task.title, &project.tasks, "");
        task.created_at = now.clone();
        task.updated_at = now.clone();

        if task.creator.is_empty() {
            task.creator = user_name.clone();
            warnings.push(format!("Task #{} \"{}\": creator auto-set to {}", idx, task.title, user_name));
        }

        // Log entry
        task.logs.push(log_entry(&user_name, "imported"));

        project.tasks.push(task);
        imported += 1;
    }

    if imported > 0 {
        state.store.put_project(project).await?;
        publish_update(&state, &id).await;
    }

    Ok(Json(ImportResponse { imported, warnings, errors, skipped }))
}

/// POST /api/projects/:id/tasks – Neue Aufgabe anlegen.
pub async fn create_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: axum::http::HeaderMap,
    Json(mut task): Json<Task>,
) -> Result<Json<ProjectDoc>, ApiError> {
    let mut project = state.store.resolve_project(&id).await?;
    if task.id.is_empty() {
        task.id = Uuid::new_v4().to_string();
    }
    task.slug = unique_task_slug(&task.title, &project.tasks, "");
    // Resolve column_slug to column_id if provided
    if task.column_id.is_empty() && !task.column_slug.is_empty() {
        let slug = task.column_slug.to_uppercase();
        if let Some(col) = project.columns.iter().find(|c| c.slug == slug) {
            task.column_id = col.id.clone();
        } else {
            return Err(ApiError::BadRequest(format!("Unknown column_slug: {}", task.column_slug)));
        }
    }
    task.column_slug.clear();
    let now = Utc::now().to_rfc3339();
    task.created_at = now.clone();
    task.updated_at = now;
    let user_name = resolve_caller(&headers, &state).await;
    if task.creator.is_empty() {
        task.creator = user_name;
    }
    project.tasks.push(task.clone());
    let updated = state.store.put_project(project).await?;
    publish_event(&state, &id, "task_created", serde_json::to_value(&task).unwrap_or_default()).await;
    Ok(Json(updated))
}

/// PUT /api/projects/:id/tasks/:task_id – Aufgabe partiell aktualisieren (Merge).
pub async fn update_task(
    State(state): State<AppState>,
    Path((id, task_id)): Path<(String, String)>,
    Json(req): Json<UpdateTaskRequest>,
) -> Result<Json<ProjectDoc>, ApiError> {
    let mut project = state.store.resolve_project(&id).await?;
    // Resolve task_id (could be slug) to real ID
    let real_task_id = project.tasks.iter()
        .find(|t| t.id == task_id || t.slug == task_id)
        .map(|t| t.id.clone())
        .ok_or_else(|| ApiError::NotFound("Task not found".into()))?;
    // Pre-compute new slug if title is changing
    let new_slug = req.title.as_ref().map(|title| {
        unique_task_slug(title, &project.tasks, &real_task_id)
    });
    if let Some(task) = project.tasks.iter_mut().find(|t| t.id == real_task_id) {
        if let Some(title) = req.title {
            task.slug = new_slug.unwrap();
            task.title = title;
        }
        if let Some(description) = req.description {
            task.description = description;
        }
        if let Some(column_id) = req.column_id {
            task.column_id = column_id;
        }
        if let Some(labels) = req.labels {
            task.labels = labels;
        }
        if let Some(worker) = req.worker {
            task.worker = worker;
        }
        if let Some(points) = req.points {
            task.points = points;
        }
        if let Some(order) = req.order {
            task.order = order;
        }
        if let Some(comments) = req.comments {
            task.comments = comments;
        }
        if let Some(logs) = req.logs {
            task.logs = logs;
        }
        if let Some(task_type) = req.task_type {
            task.task_type = task_type;
        }
        if let Some(blocks) = req.blocks {
            task.blocks = blocks;
        }
        if let Some(blocked_by) = req.blocked_by {
            task.blocked_by = blocked_by;
        }
        if let Some(parent_id) = req.parent_id {
            task.parent_id = parent_id;
        }
        if let Some(subtask_ids) = req.subtask_ids {
            task.subtask_ids = subtask_ids;
        }
        // Auto-migrate: Slug generieren falls leer
        if task.slug.is_empty() {
            task.slug = unique_task_slug(&task.title, &[], "");
        }
        task.updated_at = Utc::now().to_rfc3339();
    }
    let task_data = project.tasks.iter().find(|t| t.id == real_task_id).cloned();
    let updated = state.store.put_project(project).await?;
    if let Some(t) = task_data {
        publish_event(&state, &id, "task_updated", serde_json::to_value(&t).unwrap_or_default()).await;
    }
    Ok(Json(updated))
}

/// DELETE /api/projects/:id/tasks/:task_id – Aufgabe löschen.
pub async fn delete_task(
    State(state): State<AppState>,
    Path((id, task_id)): Path<(String, String)>,
) -> Result<Json<ProjectDoc>, ApiError> {
    let mut project = state.store.resolve_project(&id).await?;
    let real_task_id = project.tasks.iter()
        .find(|t| t.id == task_id || t.slug == task_id)
        .map(|t| t.id.clone())
        .ok_or_else(|| ApiError::NotFound("Task not found".into()))?;
    // Relationen aufräumen: blocks, blocked_by, subtask_ids, parent_id
    for task in &mut project.tasks {
        task.blocks.retain(|id| id != &real_task_id);
        task.blocked_by.retain(|id| id != &real_task_id);
        task.subtask_ids.retain(|id| id != &real_task_id);
        if task.parent_id == real_task_id {
            task.parent_id.clear();
        }
    }
    project.tasks.retain(|t| t.id != real_task_id);
    let updated = state.store.put_project(project).await?;
    publish_event(&state, &id, "task_deleted", serde_json::json!({ "task_id": task_id })).await;
    Ok(Json(updated))
}

/// POST /api/projects/:id/tasks/:task_id/move – Aufgabe in eine andere Spalte verschieben.
pub async fn move_task(
    State(state): State<AppState>,
    Path((id, task_id)): Path<(String, String)>,
    headers: axum::http::HeaderMap,
    Json(req): Json<MoveTaskRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut project = state.store.resolve_project(&id).await?;
    let user_name = resolve_caller(&headers, &state).await;
    let column_name = |col_id: &str| -> String {
        project.columns.iter()
            .find(|c| c.id == col_id)
            .map(|c| c.title.clone())
            .unwrap_or_else(|| col_id.to_string())
    };
    let real_task_id = project.tasks.iter()
        .find(|t| t.id == task_id || t.slug == task_id)
        .map(|t| t.id.clone())
        .ok_or_else(|| ApiError::NotFound("Task not found".into()))?;
    // Blocked-Check: Task darf nicht auf Done verschoben werden, wenn Blocker offen sind.
    let done_col_id = project.columns.iter().find(|c| c.title == "Done").map(|c| c.id.clone());
    if let Some(ref done_id) = done_col_id {
        if &req.column_id == done_id {
            if let Some(task) = project.tasks.iter().find(|t| t.id == real_task_id) {
                let open_blockers: Vec<&str> = task.blocked_by.iter()
                    .filter_map(|bid| project.tasks.iter().find(|t| t.id == *bid))
                    .filter(|t| Some(&t.column_id) != done_col_id.as_ref())
                    .map(|t| t.title.as_str())
                    .collect();
                if !open_blockers.is_empty() {
                    return Err(ApiError::BadRequest(format!(
                        "Task ist blockiert durch: {}",
                        open_blockers.join(", ")
                    )));
                }
            }
        }
    }
    if let Some(task) = project.tasks.iter_mut().find(|t| t.id == real_task_id) {
        let old_col = task.column_id.clone();
        let old_name = column_name(&old_col);
        let new_name = column_name(&req.column_id);
        task.previous_row = old_col;
        task.column_id = req.column_id;
        task.order = req.order.unwrap_or(task.order);
        task.updated_at = Utc::now().to_rfc3339();
        task.logs.push(log_entry(&user_name, &format!("→ {}", new_name)));
    }
    let task_data = project.tasks.iter().find(|t| t.id == real_task_id).cloned();
    state.store.put_project(project).await?;
    if let Some(t) = task_data {
        publish_event(&state, &id, "task_moved", serde_json::to_value(&t).unwrap_or_default()).await;
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// POST /api/projects/:id/tasks/batch-move – Mehrere Tasks auf einmal verschieben.
pub async fn batch_move_tasks(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: axum::http::HeaderMap,
    Json(req): Json<BatchMoveRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut project = state.store.resolve_project(&id).await?;
    let user_name = resolve_caller(&headers, &state).await;
    // Blocked-Check: keine blockierten Tasks auf Done verschieben.
    let done_col_id = project.columns.iter().find(|c| c.title == "Done").map(|c| c.id.clone());
    if let Some(ref done_id) = done_col_id {
        for m in &req.moves {
            if &m.column_id == done_id {
                if let Some(task) = project.tasks.iter().find(|t| t.id == m.task_id) {
                    let open_blockers: Vec<&str> = task.blocked_by.iter()
                        .filter_map(|bid| project.tasks.iter().find(|t| t.id == *bid))
                        .filter(|t| Some(&t.column_id) != done_col_id.as_ref())
                        .map(|t| t.title.as_str())
                        .collect();
                    if !open_blockers.is_empty() {
                        return Err(ApiError::BadRequest(format!(
                            "Task \"{}\" ist blockiert durch: {}",
                            task.title, open_blockers.join(", ")
                        )));
                    }
                }
            }
        }
    }
    let column_name = |col_id: &str| -> String {
        project.columns.iter()
            .find(|c| c.id == col_id)
            .map(|c| c.title.clone())
            .unwrap_or_else(|| col_id.to_string())
    };
    for m in &req.moves {
        if let Some(task) = project.tasks.iter_mut().find(|t| t.id == m.task_id) {
            if task.column_id != m.column_id {
                let new_name = column_name(&m.column_id);
                task.previous_row = task.column_id.clone();
                task.logs.push(log_entry(&user_name, &format!("→ {}", new_name)));
            }
            task.column_id = m.column_id.clone();
            task.order = m.order;
            task.updated_at = Utc::now().to_rfc3339();
        }
    }
    state.store.put_project(project).await?;
    publish_update(&state, &id).await;
    Ok(Json(serde_json::json!({ "ok": true, "moved": req.moves.len() })))
}
