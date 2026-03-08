// Handler für Authentifizierung.

use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;

use crate::error::ApiError;
use crate::models::*;
use crate::services::*;
use crate::state::AppState;

/// POST /auth/login – Login mit Username/Passwort.
pub async fn auth_login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Response, ApiError> {
    let user = state
        .store
        .get_user_by_username(&payload.username)
        .await
        .map_err(|_| ApiError::Unauthorized("Invalid credentials".into()))?;

    if !user.active {
        return Err(ApiError::Unauthorized("Account deactivated".into()));
    }

    if !verify_password(&payload.password, &user.password_hash) {
        return Err(ApiError::Unauthorized("Invalid credentials".into()));
    }

    let must_change = payload.password == "admin" && user.username == "admin";
    let token = create_jwt(&user, &state.jwt_secret, must_change)?;

    let cookie = format!(
        "plankton_token={}; HttpOnly; Path=/; Max-Age=28800; SameSite=Lax",
        token
    );

    let mut response = Json(serde_json::json!({
        "user_id": user.id,
        "display_name": user.display_name,
        "role": user.role,
        "must_change_password": must_change,
    }))
    .into_response();

    response
        .headers_mut()
        .insert("set-cookie", cookie.parse().unwrap());

    Ok(response)
}

/// POST /auth/logout – Logout (Cookie löschen).
pub async fn auth_logout() -> Response {
    let cookie = "plankton_token=; HttpOnly; Path=/; Max-Age=0; SameSite=Lax";
    let mut response = Json(serde_json::json!({"ok": true})).into_response();
    response
        .headers_mut()
        .insert("set-cookie", cookie.parse().unwrap());
    response
}

/// GET /auth/me – Aktuellen Benutzer abrufen.
pub async fn auth_me(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let token = extract_token_from_headers(&headers)
        .ok_or(ApiError::Unauthorized("Not authenticated".into()))?;
    let claims = validate_jwt(&token, &state.jwt_secret)?;
    Ok(Json(serde_json::json!({
        "user_id": claims.sub,
        "username": claims.username,
        "display_name": claims.display_name,
        "role": claims.role,
        "must_change_password": claims.must_change_password,
    })))
}

/// POST /auth/change-password – Passwort ändern.
pub async fn auth_change_password(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<ChangePasswordRequest>,
) -> Result<Response, ApiError> {
    let token = extract_token_from_headers(&headers)
        .ok_or(ApiError::Unauthorized("Not authenticated".into()))?;
    let claims = validate_jwt(&token, &state.jwt_secret)?;

    let mut user = state.store.get_user(&claims.sub).await?;

    if !verify_password(&payload.old_password, &user.password_hash) {
        return Err(ApiError::BadRequest("Old password incorrect".into()));
    }

    if payload.new_password.len() < 4 {
        return Err(ApiError::BadRequest(
            "Password must be at least 4 characters".into(),
        ));
    }

    user.password_hash = hash_password(&payload.new_password)?;
    user.updated_at = Utc::now().to_rfc3339();
    state.store.update_user(user.clone()).await?;

    let new_token = create_jwt(&user, &state.jwt_secret, false)?;
    let cookie = format!(
        "plankton_token={}; HttpOnly; Path=/; Max-Age=28800; SameSite=Lax",
        new_token
    );

    let mut response = Json(serde_json::json!({"ok": true})).into_response();
    response
        .headers_mut()
        .insert("set-cookie", cookie.parse().unwrap());

    Ok(response)
}
