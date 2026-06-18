// Controller für das Notification-Center.
//
// Endpunkte:
// - GET  /api/notifications          → Liste aller Notifications (neueste zuerst)
// - DELETE /api/notifications        → Alle Notifications löschen
// - DELETE /api/notifications/:id    → Einzelne Notification löschen
//
// Authentifizierung erfolgt über den globalen auth_guard.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::models::NotificationEntry;
use crate::state::AppState;

/// GET /api/notifications — Alle Notifications zurückgeben (neueste zuerst).
pub async fn list_notifications(
    State(state): State<AppState>,
) -> Result<Json<Vec<NotificationEntry>>, (StatusCode, Json<serde_json::Value>)> {
    match state.store.list_notifications().await {
        Ok(notifications) => Ok(Json(notifications)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )),
    }
}

/// DELETE /api/notifications — Alle Notifications löschen.
pub async fn clear_notifications(
    State(state): State<AppState>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    match state.store.clear_all_notifications().await {
        Ok(()) => Ok(StatusCode::NO_CONTENT),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )),
    }
}

/// DELETE /api/notifications/:id — Einzelne Notification löschen.
pub async fn delete_notification(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    match state.store.delete_notification(&id).await {
        Ok(()) => Ok(StatusCode::NO_CONTENT),
        Err(e) => {
            let status = if e.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            Err((status, Json(serde_json::json!({"error": e.to_string()}))))
        }
    }
}
