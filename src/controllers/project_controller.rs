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
    Ok(Json(state.store.list_projects().await?))
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

/// GET /api/projects/:id – Ein Projekt abrufen.
pub async fn get_project(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<GetProjectQuery>,
) -> Result<Json<ProjectDoc>, ApiError> {
    let mut project = state.store.get_project(&id).await?;
    if !query.include_archived {
        let hidden_col_ids: Vec<String> = project.columns.iter()
            .filter(|c| c.hidden)
            .map(|c| c.id.clone())
            .collect();
        project.tasks.retain(|t| !hidden_col_ids.contains(&t.column_id));
        project.columns.retain(|c| !c.hidden);
    }
    Ok(Json(project))
}

/// PUT /api/projects/:id – Vollständiges Projekt ersetzen.
pub async fn update_project(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(mut payload): Json<ProjectDoc>,
) -> Result<Json<ProjectDoc>, ApiError> {
    payload.id = id.clone();
    let current = state.store.get_project(&id).await?;
    payload.rev = current.rev;
    let updated = state.store.put_project(payload).await?;
    publish_update(&state, &id).await;
    Ok(Json(updated))
}

/// DELETE /api/projects/:id?rev=<rev> – Projekt löschen.
pub async fn delete_project(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<DeleteQuery>,
) -> Result<StatusCode, ApiError> {
    state.store.delete_project(&id, &query.rev).await?;
    publish_update(&state, &id).await;
    Ok(StatusCode::NO_CONTENT)
}
