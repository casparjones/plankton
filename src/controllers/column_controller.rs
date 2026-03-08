// Handler für Spalten-CRUD.

use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

use crate::error::ApiError;
use crate::models::*;
use crate::services::publish_update;
use crate::state::AppState;

/// POST /api/projects/:id/columns – Neue Spalte anlegen.
pub async fn create_column(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(mut column): Json<Column>,
) -> Result<Json<ProjectDoc>, ApiError> {
    let mut project = state.store.get_project(&id).await?;
    if column.id.is_empty() {
        column.id = Uuid::new_v4().to_string();
    }
    column.slug = slugify(&column.title);
    // Ensure slug uniqueness within project
    let base_slug = column.slug.clone();
    let mut counter = 1;
    while project.columns.iter().any(|c| c.slug == column.slug) {
        column.slug = format!("{}_{}", base_slug, counter);
        counter += 1;
    }
    project.columns.push(column);
    let updated = state.store.put_project(project).await?;
    publish_update(&state, &id).await;
    Ok(Json(updated))
}

/// PUT /api/projects/:id/columns/:column_id – Spalte aktualisieren.
pub async fn update_column(
    State(state): State<AppState>,
    Path((id, column_id)): Path<(String, String)>,
    Json(column): Json<Column>,
) -> Result<Json<ProjectDoc>, ApiError> {
    let mut project = state.store.get_project(&id).await?;
    // Compute unique slug before mutably borrowing
    let mut new_slug = slugify(&column.title);
    let base_slug = new_slug.clone();
    let mut counter = 1;
    while project.columns.iter().any(|c| c.id != column_id && c.slug == new_slug) {
        new_slug = format!("{}_{}", base_slug, counter);
        counter += 1;
    }
    let was_locked = project.columns.iter().find(|c| c.id == column_id).map(|c| c.locked).unwrap_or(false);
    if let Some(existing) = project.columns.iter_mut().find(|c| c.id == column_id) {
        let mut col = column;
        col.slug = new_slug;
        col.locked = was_locked;
        *existing = col;
    }
    let updated = state.store.put_project(project).await?;
    publish_update(&state, &id).await;
    Ok(Json(updated))
}

/// DELETE /api/projects/:id/columns/:column_id – Spalte und alle ihre Aufgaben löschen.
pub async fn delete_column(
    State(state): State<AppState>,
    Path((id, column_id)): Path<(String, String)>,
) -> Result<Json<ProjectDoc>, ApiError> {
    let mut project = state.store.get_project(&id).await?;
    if project.columns.iter().any(|c| c.id == column_id && c.locked) {
        return Err(ApiError::BadRequest("Locked columns cannot be deleted".into()));
    }
    project.columns.retain(|c| c.id != column_id);
    project.tasks.retain(|t| t.column_id != column_id);
    let updated = state.store.put_project(project).await?;
    publish_update(&state, &id).await;
    Ok(Json(updated))
}
