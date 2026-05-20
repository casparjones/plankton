// Incoming Webhook Controller
//
// POST /webhook/projects/:slug/tasks/:task_id/move
//   Authorization: Bearer <plk_...>
//   Body: {"column": "DONE"}   (Column-Slug oder Column-Titel)
//
// Bewegt einen Task in die angegebene Spalte. Selbe Auth-Logik wie
// bestehende API (AgentToken mit `plk_`-Präfix oder JWT).

use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use serde::Deserialize;

use crate::error::ApiError;
use crate::models::log_entry;
use crate::services::{extract_token_from_headers, validate_jwt};
use crate::state::AppState;

/// Request-Body für den Incoming Webhook.
#[derive(Debug, Deserialize)]
pub struct IncomingMoveRequest {
    /// Column-Slug (z.B. "DONE") oder Column-Titel (z.B. "Done")
    pub column: String,
}

/// Caller-Identität aus Headers auflösen (JWT oder Agent-Token).
async fn resolve_caller_name(
    headers: &axum::http::HeaderMap,
    state: &AppState,
) -> Result<String, ApiError> {
    if let Some(t) = extract_token_from_headers(headers) {
        if let Ok(claims) = validate_jwt(&t, &state.jwt_secret) {
            return Ok(claims.display_name);
        }
    }
    if let Some(bearer) = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
    {
        if let Ok(agent_token) = state.store.get_token_by_value(bearer).await {
            return Ok(agent_token.name);
        }
    }
    Err(ApiError::Unauthorized("Invalid or missing token".into()))
}

/// POST /webhook/projects/:slug/tasks/:task_id/move
pub async fn incoming_move_task(
    State(state): State<AppState>,
    Path((slug, task_id)): Path<(String, String)>,
    headers: axum::http::HeaderMap,
    Json(req): Json<IncomingMoveRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let caller = resolve_caller_name(&headers, &state).await?;

    let mut project = state.store.resolve_project(&slug).await?;

    // Column per Slug oder Titel finden
    let column_slug_upper = req.column.trim().to_uppercase();
    let target_col_id = project
        .columns
        .iter()
        .find(|c| {
            c.slug.to_uppercase() == column_slug_upper
                || c.title.to_uppercase() == column_slug_upper
        })
        .map(|c| c.id.clone())
        .ok_or_else(|| ApiError::BadRequest(format!("Unknown column: {}", req.column)))?;

    let target_col_title = project
        .columns
        .iter()
        .find(|c| c.id == target_col_id)
        .map(|c| c.title.clone())
        .unwrap_or_else(|| req.column.clone());

    // Task finden (per ID oder Slug)
    let real_task_id = project
        .tasks
        .iter()
        .find(|t| t.id == task_id || t.slug == task_id)
        .map(|t| t.id.clone())
        .ok_or_else(|| ApiError::NotFound("Task not found".into()))?;

    // Task verschieben
    if let Some(task) = project.tasks.iter_mut().find(|t| t.id == real_task_id) {
        task.previous_row = task.column_id.clone();
        task.column_id = target_col_id.clone();
        task.updated_at = Utc::now().to_rfc3339();
        task.logs.push(log_entry(
            &caller,
            &format!("→ {} (webhook)", target_col_title),
        ));
    }

    // Outgoing Webhook: task.moved Event auslösen
    let task_snapshot = project.tasks.iter().find(|t| t.id == real_task_id).cloned();

    let webhook_url = project.webhook_url.clone();
    let project_slug = project.slug.clone();

    state.store.put_project(project).await?;

    // SSE-Event senden
    crate::services::publish_event(
        &state,
        &slug,
        "task_moved",
        task_snapshot
            .as_ref()
            .and_then(|t| serde_json::to_value(t).ok())
            .unwrap_or_default(),
    )
    .await;

    // Outgoing Webhook feuern
    if let Some(task) = task_snapshot {
        crate::services::webhook_service::dispatch_webhook(
            state.http_client.clone(),
            webhook_url,
            crate::services::webhook_service::WebhookEvent {
                event: "task.moved".to_string(),
                project: project_slug,
                task: crate::services::webhook_service::WebhookTaskInfo {
                    id: task.id.clone(),
                    title: task.title.clone(),
                    column: target_col_title,
                    worker: task.worker.clone(),
                },
                ts: Utc::now().to_rfc3339(),
            },
        );
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}
