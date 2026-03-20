// Handler für Git-Repository-Konfiguration pro Projekt.

use axum::{
    extract::{Path, State},
    Json,
};

use serde::Serialize;

use crate::error::ApiError;
use crate::models::project::GitConfig;
use crate::services::{perform_git_sync, publish_update};
use crate::state::AppState;

/// GET /api/projects/:id/git – Git-Konfiguration abrufen.
pub async fn get_git_config(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Option<GitConfig>>, ApiError> {
    let project = state.store.resolve_project(&id).await?;
    Ok(Json(project.git))
}

/// PUT /api/projects/:id/git – Git-Konfiguration setzen oder aktualisieren.
pub async fn update_git_config(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(config): Json<GitConfig>,
) -> Result<Json<GitConfig>, ApiError> {
    let mut project = state.store.resolve_project(&id).await?;
    project.git = Some(config.clone());
    state.store.put_project(project).await?;
    publish_update(&state, &id).await;
    Ok(Json(config))
}

/// Antwort für den Sync-Endpunkt.
#[derive(Serialize)]
pub struct GitSyncResponse {
    pub success: bool,
    pub message: String,
}

/// POST /api/projects/:id/git/sync – Manueller Git-Sync auslösen.
pub async fn git_sync(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<GitSyncResponse>, ApiError> {
    match perform_git_sync(&state, &id).await {
        Ok(()) => Ok(Json(GitSyncResponse {
            success: true,
            message: "Git-Sync erfolgreich".into(),
        })),
        Err(err) => Ok(Json(GitSyncResponse {
            success: false,
            message: err,
        })),
    }
}
