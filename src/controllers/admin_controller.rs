// Handler für Admin-Endpunkte (User- und Token-Verwaltung).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use uuid::Uuid;

use crate::error::ApiError;
use crate::models::auth::{hash_token_secret, TokenScope};
use crate::models::*;
use crate::services::*;
use crate::state::AppState;

// ---- Öffentliche User-Liste (nur username + display_name) ----

/// GET /api/users – Alle aktiven Benutzer (öffentlich, minimale Daten).
pub async fn public_list_users(
    State(state): State<AppState>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let users = state.store.list_users().await?;
    Ok(Json(
        users
            .iter()
            .filter(|u| u.active)
            .map(|u| {
                serde_json::json!({
                    "username": u.username,
                    "display_name": u.display_name,
                })
            })
            .collect(),
    ))
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

/// GET /api/admin/tokens – Alle Agent-Tokens auflisten (kein Secret/Hash im Response).
pub async fn admin_list_tokens(
    State(state): State<AppState>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let tokens = state.store.list_tokens().await?;
    Ok(Json(tokens.iter().map(token_to_list_json).collect()))
}

/// Serialisiert einen AgentToken für Listen-Responses — kein Secret, kein Hash.
fn token_to_list_json(t: &AgentToken) -> serde_json::Value {
    serde_json::json!({
        "id": t.id,
        "name": t.name,
        "description": t.description,
        "role": t.role,
        "active": t.active,
        "created_at": t.created_at,
        "creator": t.creator,
        "last_used": t.last_used,
        "scope": t.scope,
        "expires_at": t.expires_at,
    })
}

/// POST /api/admin/tokens – Neuen Agent-Token erstellen.
/// Das Token-Secret wird **nur einmal** im Response zurückgegeben.
/// In der DB wird ausschließlich der SHA-256-Hash gespeichert.
pub async fn admin_create_token(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<CreateTokenRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    // Ersteller aus JWT ermitteln (falls vorhanden)
    let creator = extract_token_from_headers(&headers)
        .and_then(|t| validate_jwt(&t, &state.jwt_secret).ok())
        .map(|c| c.username)
        .unwrap_or_else(|| "admin".to_string());

    // Secret generieren (Klartext — wird nur im Response zurückgegeben)
    let secret = generate_agent_token();
    let token_hash = hash_token_secret(&secret);

    let token = AgentToken {
        id: Uuid::new_v4().to_string(),
        name: payload.name,
        token_hash,
        role: payload.role,
        active: true,
        created_at: Utc::now().to_rfc3339(),
        description: payload.description,
        creator,
        last_used: None,
        scope: payload.scope.unwrap_or(TokenScope::Global),
        expires_at: payload.expires_at,
    };
    let created = state.store.create_token(token).await?;

    // Secret EINMALIG im Response zurückgeben (201 Created)
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id": created.id,
            "name": created.name,
            "secret": secret,           // Einmalig — wird nicht erneut ausgegeben
            "role": created.role,
            "active": created.active,
            "created_at": created.created_at,
            "description": created.description,
            "creator": created.creator,
            "scope": created.scope,
            "expires_at": created.expires_at,
        })),
    ))
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
    if let Some(description) = payload.description {
        token.description = description;
    }
    let updated = state.store.update_token(token).await?;
    Ok(Json(token_to_list_json(&updated)))
}

/// DELETE /api/admin/tokens/:token_id – Token löschen.
pub async fn admin_delete_token(
    State(state): State<AppState>,
    Path(token_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.store.delete_token(&token_id).await?;
    Ok(Json(serde_json::json!({"ok": true})))
}

// ---- System-Status ----

/// GET /api/admin/system-status – Maintenance-Job Status (letzter/nächster Lauf).
pub async fn admin_system_status(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    const INTERVAL_SECS: i64 = 3600;

    let last_run = *state.last_maintenance_run.read().await;

    let next_run = match last_run {
        Some(last) => last + chrono::Duration::seconds(INTERVAL_SECS),
        None => state.started_at + chrono::Duration::seconds(INTERVAL_SECS),
    };

    Ok(Json(serde_json::json!({
        "last_maintenance_run": last_run.map(|t| t.to_rfc3339()),
        "next_maintenance_run": next_run.to_rfc3339(),
        "interval_seconds": INTERVAL_SECS,
    })))
}
