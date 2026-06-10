// Handler für File-Attachment-Endpunkte.
// Nur registriert wenn S3 konfiguriert ist (attachment_store.is_some()).

use axum::{
    extract::{Multipart, Path, State},
    Json,
};
use uuid::Uuid;

use crate::error::ApiError;
use crate::models::{AttachmentRef, Task};
use crate::services::{extract_token_from_headers, validate_jwt};
use crate::state::AppState;

/// S3-Key-Format: {project_id}/{task_id}/{attachment_id}/{filename}
fn s3_key(project_id: &str, task_id: &str, attachment_id: &str, filename: &str) -> String {
    format!("{}/{}/{}/{}", project_id, task_id, attachment_id, filename)
}

fn resolve_caller_sync(headers: &axum::http::HeaderMap, state: &AppState) -> Option<String> {
    if let Some(t) = extract_token_from_headers(headers) {
        if let Ok(claims) = validate_jwt(&t, &state.jwt_secret) {
            return Some(claims.display_name);
        }
    }
    if let Some(bearer) = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
    {
        // Agent-Token: name aus dem Token (sync-check via blocking ist hier nicht möglich,
        // aber der auth_guard hat schon geprüft – wir geben "agent" zurück).
        let _ = bearer;
        return Some("agent".to_string());
    }
    None
}

/// POST /api/projects/:id/tasks/:task_id/attachments
/// Multipart-Upload: Datei → S3, Metadaten → Task.
pub async fn upload_attachment(
    State(state): State<AppState>,
    Path((project_id, task_id)): Path<(String, String)>,
    headers: axum::http::HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<AttachmentRef>, ApiError> {
    let store = state.attachment_store.as_ref().ok_or_else(|| {
        ApiError::InternalError("File uploads not configured on this server".into())
    })?;

    // Datei aus Multipart lesen
    let mut filename = String::new();
    let mut data: Vec<u8> = Vec::new();
    let mut mime_type = "application/octet-stream".to_string();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(format!("multipart error: {e}")))?
    {
        if field.name() == Some("file") {
            filename = field.file_name().unwrap_or("upload").to_string();
            mime_type = field
                .content_type()
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    mime_guess::from_path(&filename)
                        .first_or_octet_stream()
                        .to_string()
                });
            data = field
                .bytes()
                .await
                .map_err(|e| ApiError::BadRequest(format!("read error: {e}")))?
                .to_vec();
            break;
        }
    }

    if filename.is_empty() || data.is_empty() {
        return Err(ApiError::BadRequest(
            "multipart field 'file' with filename required".into(),
        ));
    }

    let attachment_id = Uuid::new_v4().to_string();
    let key = s3_key(&project_id, &task_id, &attachment_id, &filename);
    let size_bytes = data.len() as i64;

    let url = store.upload(&key, data, &mime_type).await?;

    let att = AttachmentRef {
        id: attachment_id,
        filename,
        url,
        mime_type,
        size_bytes,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    // Attachment-Ref im Task persistieren
    let _caller = resolve_caller_sync(&headers, &state);
    let lock = state.get_project_write_lock(&project_id).await;
    let _guard = lock.lock().await;

    let mut project = state.store.resolve_project(&project_id).await?;
    let task = project
        .tasks
        .iter_mut()
        .find(|t| t.id == task_id || t.slug == task_id)
        .ok_or_else(|| ApiError::NotFound(format!("task {task_id} not found")))?;

    task.attachments.push(att.clone());
    state.store.put_project(project).await?;

    Ok(Json(att))
}

/// GET /api/projects/:id/tasks/:task_id/attachments
/// Liste aller Anhänge des Tasks.
pub async fn list_attachments(
    State(state): State<AppState>,
    Path((project_id, task_id)): Path<(String, String)>,
) -> Result<Json<Vec<AttachmentRef>>, ApiError> {
    let project = state.store.resolve_project(&project_id).await?;
    let task = project
        .tasks
        .iter()
        .find(|t| t.id == task_id || t.slug == task_id)
        .ok_or_else(|| ApiError::NotFound(format!("task {task_id} not found")))?;

    Ok(Json(task.attachments.clone()))
}

/// GET /api/projects/:id/tasks/:task_id/attachments/:attachment_id
/// Redirect zu Presigned S3 Download-URL (TTL 1h).
pub async fn download_attachment(
    State(state): State<AppState>,
    Path((project_id, task_id, attachment_id)): Path<(String, String, String)>,
) -> Result<axum::response::Redirect, ApiError> {
    let store = state.attachment_store.as_ref().ok_or_else(|| {
        ApiError::InternalError("File uploads not configured on this server".into())
    })?;

    let project = state.store.resolve_project(&project_id).await?;
    let task = project
        .tasks
        .iter()
        .find(|t| t.id == task_id || t.slug == task_id)
        .ok_or_else(|| ApiError::NotFound(format!("task {task_id} not found")))?;

    let att = task
        .attachments
        .iter()
        .find(|a| a.id == attachment_id)
        .ok_or_else(|| ApiError::NotFound(format!("attachment {attachment_id} not found")))?;

    // S3-Key aus URL rekonstruieren oder Presigned URL direkt verwenden
    let key = s3_key(&project_id, &task_id, &attachment_id, &att.filename);
    let url = store.download_url(&key, 3600).await?;

    Ok(axum::response::Redirect::temporary(&url))
}

/// DELETE /api/projects/:id/tasks/:task_id/attachments/:attachment_id
pub async fn delete_attachment(
    State(state): State<AppState>,
    Path((project_id, task_id, attachment_id)): Path<(String, String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let store = state.attachment_store.as_ref().ok_or_else(|| {
        ApiError::InternalError("File uploads not configured on this server".into())
    })?;

    let lock = state.get_project_write_lock(&project_id).await;
    let _guard = lock.lock().await;

    let mut project = state.store.resolve_project(&project_id).await?;
    let task: &mut Task = project
        .tasks
        .iter_mut()
        .find(|t| t.id == task_id || t.slug == task_id)
        .ok_or_else(|| ApiError::NotFound(format!("task {task_id} not found")))?;

    let idx = task
        .attachments
        .iter()
        .position(|a| a.id == attachment_id)
        .ok_or_else(|| ApiError::NotFound(format!("attachment {attachment_id} not found")))?;

    let att = task.attachments.remove(idx);
    state.store.put_project(project).await?;

    // S3-Objekt löschen (nach DB-Update, damit bei Fehler kein Ghost-Eintrag bleibt)
    let key = s3_key(&project_id, &task_id, &attachment_id, &att.filename);
    store.delete(&key).await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}
