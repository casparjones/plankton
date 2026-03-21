// Handler für Projekt-CRUD.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::error::ApiError;
use crate::models::*;
use crate::services::publish_update;
use crate::state::AppState;

/// GET /api/projects – Alle Projekte auflisten.
pub async fn list_projects(State(state): State<AppState>) -> Result<Json<Vec<ProjectDoc>>, ApiError> {
    let mut projects = state.store.list_projects().await?;
    // Auto-migrate: Slugs generieren für Projekte ohne Slug.
    let mut existing_slugs: Vec<String> = projects.iter()
        .filter(|p| !p.slug.is_empty())
        .map(|p| p.slug.clone())
        .collect();
    for project in &mut projects {
        if project.slug.is_empty() {
            let base_slug = project_slugify(&project.title);
            let slug_refs: Vec<&str> = existing_slugs.iter().map(|s| s.as_str()).collect();
            project.slug = unique_slug(&base_slug, &slug_refs);
            existing_slugs.push(project.slug.clone());
            let _ = state.store.put_project(project.clone()).await;
        }
    }
    Ok(Json(projects))
}

/// POST /api/projects – Neues Projekt anlegen.
pub async fn create_project(
    State(state): State<AppState>,
    Json(mut payload): Json<ProjectDoc>,
) -> Result<Json<ProjectDoc>, ApiError> {
    if payload.id.is_empty() {
        payload.id = Uuid::new_v4().to_string();
    }
    payload.rev = None;
    // Auto-generate project slug from title
    if payload.slug.is_empty() {
        let base_slug = project_slugify(&payload.title);
        let existing = state.store.list_projects().await?;
        let existing_slugs: Vec<&str> = existing.iter().map(|p| p.slug.as_str()).collect();
        payload.slug = unique_slug(&base_slug, &existing_slugs);
    }
    // Ensure all columns have slugs
    for col in &mut payload.columns {
        if col.slug.is_empty() {
            col.slug = slugify(&col.title);
        }
    }
    let created = state.store.create_project(payload).await?;
    publish_update(&state, &created.id).await;
    Ok(Json(created))
}

/// Erzeugt einen eindeutigen Slug durch Anhängen eines Zählers.
fn unique_slug(base: &str, existing: &[&str]) -> String {
    if !existing.contains(&base) {
        return base.to_string();
    }
    for i in 2.. {
        let candidate = format!("{base}-{i}");
        if !existing.contains(&candidate.as_str()) {
            return candidate;
        }
    }
    unreachable!()
}

/// GET /api/projects/:id – Ein Projekt abrufen (akzeptiert UUID oder Slug).
pub async fn get_project(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<GetProjectQuery>,
) -> Result<Json<ProjectDoc>, ApiError> {
    let mut project = state.store.resolve_project(&id).await?;
    // Auto-migrate: Slug generieren für Projekte ohne Slug.
    if project.slug.is_empty() {
        let base_slug = project_slugify(&project.title);
        let existing = state.store.list_projects().await?;
        let existing_slugs: Vec<&str> = existing.iter()
            .filter(|p| p.id != project.id)
            .map(|p| p.slug.as_str())
            .collect();
        project.slug = unique_slug(&base_slug, &existing_slugs);
        let _ = state.store.put_project(project.clone()).await;
    }
    // Auto-migrate: Task-Slugs generieren für Tasks ohne Slug.
    let needs_task_slugs = project.tasks.iter().any(|t| t.slug.is_empty());
    if needs_task_slugs {
        let mut existing_slugs: Vec<String> = Vec::new();
        for task in &mut project.tasks {
            if task.slug.is_empty() {
                let base = project_slugify(&task.title);
                let refs: Vec<&str> = existing_slugs.iter().map(|s| s.as_str()).collect();
                task.slug = unique_slug(&base, &refs);
            }
            existing_slugs.push(task.slug.clone());
        }
        let _ = state.store.put_project(project.clone()).await;
    }
    if !query.include_archived {
        let hidden_col_ids: Vec<String> = project.columns.iter()
            .filter(|c| c.hidden)
            .map(|c| c.id.clone())
            .collect();
        project.tasks.retain(|t| !hidden_col_ids.contains(&t.column_id));
        project.columns.retain(|c| !c.hidden);
    }
    // Sort tasks based on query parameter (default: order).
    match query.sort.as_deref() {
        Some("title") => project.tasks.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase())),
        Some("created") => project.tasks.sort_by(|a, b| a.created_at.cmp(&b.created_at)),
        Some("updated") => project.tasks.sort_by(|a, b| b.updated_at.cmp(&a.updated_at)),
        Some("points") => project.tasks.sort_by(|a, b| b.points.cmp(&a.points)),
        _ => project.tasks.sort_by_key(|t| t.order),
    }
    // Group epics with their subtasks: epic first, then its children in order.
    if query.group_epics {
        let mut grouped: Vec<Task> = Vec::with_capacity(project.tasks.len());
        let mut placed: std::collections::HashSet<String> = std::collections::HashSet::new();
        // Collect subtask IDs for quick lookup.
        let subtask_parent: std::collections::HashMap<String, String> = project.tasks.iter()
            .filter(|t| !t.parent_id.is_empty())
            .map(|t| (t.id.clone(), t.parent_id.clone()))
            .collect();
        for task in &project.tasks {
            if placed.contains(&task.id) { continue; }
            // Skip subtasks here; they'll be placed after their parent.
            if subtask_parent.contains_key(&task.id) { continue; }
            grouped.push(task.clone());
            placed.insert(task.id.clone());
            // If this is an epic, insert its subtasks right after.
            if !task.subtask_ids.is_empty() {
                // Maintain order among subtasks.
                let mut subs: Vec<&Task> = project.tasks.iter()
                    .filter(|t| t.parent_id == task.id && !placed.contains(&t.id))
                    .collect();
                subs.sort_by_key(|t| t.order);
                for sub in subs {
                    grouped.push(sub.clone());
                    placed.insert(sub.id.clone());
                }
            }
        }
        // Append any remaining tasks (orphaned subtasks whose parent wasn't found).
        for task in &project.tasks {
            if !placed.contains(&task.id) {
                grouped.push(task.clone());
            }
        }
        project.tasks = grouped;
    }
    Ok(Json(project))
}

/// PUT /api/projects/:id – Vollständiges Projekt ersetzen (akzeptiert UUID oder Slug).
pub async fn update_project(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(mut payload): Json<ProjectDoc>,
) -> Result<Json<ProjectDoc>, ApiError> {
    let real_id = state.store.resolve_project_id(&id).await?;
    payload.id = real_id.clone();
    let current = state.store.get_project(&real_id).await?;
    payload.rev = current.rev;
    // Re-generate slug if title changed
    if payload.slug.is_empty() || current.title != payload.title {
        let base_slug = project_slugify(&payload.title);
        let existing = state.store.list_projects().await?;
        let existing_slugs: Vec<&str> = existing.iter()
            .filter(|p| p.id != real_id)
            .map(|p| p.slug.as_str())
            .collect();
        payload.slug = unique_slug(&base_slug, &existing_slugs);
    }
    let updated = state.store.put_project(payload).await?;
    publish_update(&state, &real_id).await;
    Ok(Json(updated))
}

/// DELETE /api/projects/:id?rev=<rev> – Projekt löschen (akzeptiert UUID oder Slug).
pub async fn delete_project(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<DeleteQuery>,
) -> Result<StatusCode, ApiError> {
    let real_id = state.store.resolve_project_id(&id).await?;
    state.store.delete_project(&real_id, &query.rev).await?;
    publish_update(&state, &real_id).await;
    Ok(StatusCode::NO_CONTENT)
}
