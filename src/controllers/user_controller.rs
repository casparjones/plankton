// Handler für Projekt-Nutzer-CRUD.

use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

use crate::error::ApiError;
use crate::models::*;
use crate::services::publish_update;
use crate::state::AppState;

/// POST /api/projects/:id/users – Neuen Nutzer zum Projekt hinzufügen.
pub async fn create_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(mut user): Json<User>,
) -> Result<Json<ProjectDoc>, ApiError> {
    let mut project = state.store.get_project(&id).await?;
    if user.id.is_empty() {
        user.id = Uuid::new_v4().to_string();
    }
    project.users.push(user);
    let updated = state.store.put_project(project).await?;
    publish_update(&state, &id).await;
    Ok(Json(updated))
}

/// PUT /api/projects/:id/users/:user_id – Nutzer aktualisieren.
pub async fn update_user(
    State(state): State<AppState>,
    Path((id, user_id)): Path<(String, String)>,
    Json(user): Json<User>,
) -> Result<Json<ProjectDoc>, ApiError> {
    let mut project = state.store.get_project(&id).await?;
    if let Some(existing) = project.users.iter_mut().find(|u| u.id == user_id) {
        *existing = user;
    }
    let updated = state.store.put_project(project).await?;
    publish_update(&state, &id).await;
    Ok(Json(updated))
}

/// DELETE /api/projects/:id/users/:user_id – Nutzer aus dem Projekt entfernen.
pub async fn delete_user(
    State(state): State<AppState>,
    Path((id, user_id)): Path<(String, String)>,
) -> Result<Json<ProjectDoc>, ApiError> {
    let mut project = state.store.get_project(&id).await?;
    project.users.retain(|u| u.id != user_id);
    for task in &mut project.tasks {
        task.assignee_ids.retain(|uid| uid != &user_id);
    }
    let updated = state.store.put_project(project).await?;
    publish_update(&state, &id).await;
    Ok(Json(updated))
}
