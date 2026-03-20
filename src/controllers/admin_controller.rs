// Handler für Admin-Endpunkte (User- und Token-Verwaltung).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use uuid::Uuid;

use crate::error::ApiError;
use crate::models::*;
use crate::services::*;
use crate::state::AppState;

// ---- Öffentliche User-Liste (nur username + display_name) ----

/// GET /api/users – Alle aktiven Benutzer (öffentlich, minimale Daten).
pub async fn public_list_users(
    State(state): State<AppState>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let users = state.store.list_users().await?;
    Ok(Json(users.iter()
        .filter(|u| u.active)
        .map(|u| serde_json::json!({
            "username": u.username,
            "display_name": u.display_name,
        }))
        .collect()))
}

// ---- User-Verwaltung ----

/// GET /api/admin/users – Alle Benutzer auflisten.
pub async fn admin_list_users(
    State(state): State<AppState>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let users = state.store.list_users().await?;
    Ok(Json(users.iter().map(user_to_json).collect()))
}

/// POST /api/admin/users – Neuen Benutzer anlegen.
pub async fn admin_create_user(
    State(state): State<AppState>,
    Json(payload): Json<CreateAuthUserRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if state
        .store
        .get_user_by_username(&payload.username.to_lowercase())
        .await
        .is_ok()
    {
        return Err(ApiError::Conflict(format!(
            "Username '{}' already exists",
            payload.username
        )));
    }

    let now = Utc::now().to_rfc3339();
    let user = AuthUser {
        id: Uuid::new_v4().to_string(),
        username: payload.username.to_lowercase(),
        display_name: payload.display_name,
        password_hash: hash_password(&payload.password)?,
        role: payload.role,
        created_at: now.clone(),
        updated_at: now,
        active: true,
    };

    let created = state.store.create_user(user).await?;
    Ok(Json(user_to_json(&created)))
}

/// PUT /api/admin/users/:user_id – Benutzer aktualisieren.
pub async fn admin_update_user(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    Json(payload): Json<UpdateAuthUserRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut user = state.store.get_user(&user_id).await?;

    if let Some(name) = payload.display_name {
        user.display_name = name;
    }
    if let Some(role) = payload.role {
        user.role = role;
    }
    if let Some(active) = payload.active {
        user.active = active;
    }
    user.updated_at = Utc::now().to_rfc3339();

    let updated = state.store.update_user(user).await?;
    Ok(Json(user_to_json(&updated)))
}

/// DELETE /api/admin/users/:user_id – Benutzer löschen.
pub async fn admin_delete_user(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    headers: axum::http::HeaderMap,
) -> Result<StatusCode, ApiError> {
    if let Some(token) = extract_token_from_headers(&headers) {
        if let Ok(claims) = validate_jwt(&token, &state.jwt_secret) {
            if claims.sub == user_id {
                return Err(ApiError::BadRequest(
                    "Cannot delete your own account".into(),
                ));
            }
        }
    }
    state.store.delete_user(&user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// PUT /api/admin/users/:user_id/password – Passwort zurücksetzen.
pub async fn admin_reset_password(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    Json(payload): Json<ResetPasswordRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut user = state.store.get_user(&user_id).await?;
    user.password_hash = hash_password(&payload.password)?;
    user.updated_at = Utc::now().to_rfc3339();
    state.store.update_user(user).await?;
    Ok(Json(serde_json::json!({"ok": true})))
}

// ---- Token-Verwaltung ----

/// GET /api/admin/tokens – Alle Agent-Tokens auflisten.
pub async fn admin_list_tokens(
    State(state): State<AppState>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let tokens = state.store.list_tokens().await?;
    Ok(Json(
        tokens
            .iter()
            .map(|t| {
                serde_json::json!({
                    "id": t.id,
                    "name": t.name,
                    "token": t.token,
                    "role": t.role,
                    "active": t.active,
                    "created_at": t.created_at,
                })
            })
            .collect(),
    ))
}

/// POST /api/admin/tokens – Neuen Agent-Token erstellen.
pub async fn admin_create_token(
    State(state): State<AppState>,
    Json(payload): Json<CreateTokenRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let token = AgentToken {
        id: Uuid::new_v4().to_string(),
        name: payload.name,
        token: generate_agent_token(),
        role: payload.role,
        active: true,
        created_at: Utc::now().to_rfc3339(),
    };
    let created = state.store.create_token(token).await?;
    Ok(Json(serde_json::json!({
        "id": created.id,
        "name": created.name,
        "token": created.token,
        "role": created.role,
        "active": created.active,
        "created_at": created.created_at,
    })))
}

/// PUT /api/admin/tokens/:token_id – Token aktualisieren.
pub async fn admin_update_token(
    State(state): State<AppState>,
    Path(token_id): Path<String>,
    Json(payload): Json<UpdateTokenRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut token = state.store.get_token(&token_id).await?;
    if let Some(name) = payload.name {
        token.name = name;
    }
    if let Some(role) = payload.role {
        token.role = role;
    }
    if let Some(active) = payload.active {
        token.active = active;
    }
    let updated = state.store.update_token(token).await?;
    Ok(Json(serde_json::json!({
        "id": updated.id,
        "name": updated.name,
        "role": updated.role,
        "active": updated.active,
        "created_at": updated.created_at,
    })))
}

/// DELETE /api/admin/tokens/:token_id – Token löschen.
pub async fn admin_delete_token(
    State(state): State<AppState>,
    Path(token_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.store.delete_token(&token_id).await?;
    Ok(Json(serde_json::json!({"ok": true})))
}
