// Handler für Task-CRUD und Task-Move.

use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{Local, Utc};
use uuid::Uuid;

use crate::error::ApiError;
use crate::models::*;
use crate::services::{extract_token_from_headers, publish_update, validate_jwt};
use crate::state::AppState;

/// POST /api/projects/:id/import – Mehrere Tasks auf einmal importieren.
pub async fn import_tasks(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: axum::http::HeaderMap,
    Json(req): Json<ImportRequest>,
) -> Result<Json<ImportResponse>, ApiError> {
    let mut project = state.store.get_project(&id).await?;
    let user_name = extract_token_from_headers(&headers)
        .and_then(|t| validate_jwt(&t, &state.jwt_secret).ok())
        .map(|c| c.display_name)
        .unwrap_or_else(|| "anonymous".to_string());

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
        task.created_at = now.clone();
        task.updated_at = now.clone();

        if task.creator.is_empty() {
            task.creator = user_name.clone();
            warnings.push(format!("Task #{} \"{}\": creator auto-set to {}", idx, task.title, user_name));
        }

        // Log entry
        task.logs.push(format!("{} imported via Issue Import [{}]", today, user_name));

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
    let mut project = state.store.get_project(&id).await?;
    if task.id.is_empty() {
        task.id = Uuid::new_v4().to_string();
    }
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
    let user_name = extract_token_from_headers(&headers)
        .and_then(|t| validate_jwt(&t, &state.jwt_secret).ok())
        .map(|c| c.display_name)
        .unwrap_or_else(|| "anonymous".to_string());
    if task.creator.is_empty() {
        task.creator = user_name;
    }
    project.tasks.push(task);
    let updated = state.store.put_project(project).await?;
    publish_update(&state, &id).await;
    Ok(Json(updated))
}

/// PUT /api/projects/:id/tasks/:task_id – Aufgabe partiell aktualisieren (Merge).
pub async fn update_task(
    State(state): State<AppState>,
    Path((id, task_id)): Path<(String, String)>,
    Json(req): Json<UpdateTaskRequest>,
) -> Result<Json<ProjectDoc>, ApiError> {
    let mut project = state.store.get_project(&id).await?;
    if let Some(task) = project.tasks.iter_mut().find(|t| t.id == task_id) {
        if let Some(title) = req.title {
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
        task.updated_at = Utc::now().to_rfc3339();
    } else {
        return Err(ApiError::NotFound("Task not found".into()));
    }
    let updated = state.store.put_project(project).await?;
    publish_update(&state, &id).await;
    Ok(Json(updated))
}

/// DELETE /api/projects/:id/tasks/:task_id – Aufgabe löschen.
pub async fn delete_task(
    State(state): State<AppState>,
    Path((id, task_id)): Path<(String, String)>,
) -> Result<Json<ProjectDoc>, ApiError> {
    let mut project = state.store.get_project(&id).await?;
    project.tasks.retain(|t| t.id != task_id);
    let updated = state.store.put_project(project).await?;
    publish_update(&state, &id).await;
    Ok(Json(updated))
}

/// POST /api/projects/:id/tasks/:task_id/move – Aufgabe in eine andere Spalte verschieben.
pub async fn move_task(
    State(state): State<AppState>,
    Path((id, task_id)): Path<(String, String)>,
    headers: axum::http::HeaderMap,
    Json(req): Json<MoveTaskRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut project = state.store.get_project(&id).await?;
    let user_name = extract_token_from_headers(&headers)
        .and_then(|t| validate_jwt(&t, &state.jwt_secret).ok())
        .map(|c| c.display_name)
        .unwrap_or_else(|| "anonymous".to_string());
    let column_name = |col_id: &str| -> String {
        project.columns.iter()
            .find(|c| c.id == col_id)
            .map(|c| c.title.clone())
            .unwrap_or_else(|| col_id.to_string())
    };
    if let Some(task) = project.tasks.iter_mut().find(|t| t.id == task_id) {
        let old_col = task.column_id.clone();
        let old_name = column_name(&old_col);
        let new_name = column_name(&req.column_id);
        task.previous_row = old_col;
        task.column_id = req.column_id;
        task.order = req.order.unwrap_or(task.order);
        task.updated_at = Utc::now().to_rfc3339();
        let log = format!("[{}] {} moved from {} to {}",
            user_name, Local::now().format("%Y-%m-%d %H:%M"), old_name, new_name);
        task.logs.push(log);
    } else {
        return Err(ApiError::NotFound("Task not found".into()));
    }
    state.store.put_project(project).await?;
    publish_update(&state, &id).await;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// POST /api/projects/:id/tasks/batch-move – Mehrere Tasks auf einmal verschieben.
pub async fn batch_move_tasks(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<BatchMoveRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut project = state.store.get_project(&id).await?;
    for m in &req.moves {
        if let Some(task) = project.tasks.iter_mut().find(|t| t.id == m.task_id) {
            task.column_id = m.column_id.clone();
            task.order = m.order;
            task.updated_at = Utc::now().to_rfc3339();
        }
    }
    state.store.put_project(project).await?;
    publish_update(&state, &id).await;
    Ok(Json(serde_json::json!({ "ok": true, "moved": req.moves.len() })))
}
