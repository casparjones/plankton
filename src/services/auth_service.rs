// Auth-Hilfsfunktionen: Passwort-Hashing, JWT, Token-Extraktion.

use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header as JwtHeader, Validation};
use rand::rngs::OsRng;

use crate::error::ApiError;
use crate::models::{AuthUser, Claims};

/// Passwort mit Argon2 hashen.
pub fn hash_password(password: &str) -> Result<String, ApiError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| ApiError::BadRequest(format!("Hash error: {e}")))?;
    Ok(hash.to_string())
}

/// Passwort gegen Hash verifizieren.
pub fn verify_password(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash)
        .ok()
        .map(|h| Argon2::default().verify_password(password.as_bytes(), &h).is_ok())
        .unwrap_or(false)
}

/// JWT-Token erstellen (Standard: 8 Stunden).
pub fn create_jwt(user: &AuthUser, secret: &str, must_change_pw: bool) -> Result<String, ApiError> {
    create_jwt_with_duration(user, secret, must_change_pw, chrono::Duration::hours(8))
}

/// JWT-Token mit konfigurierbarer Gültigkeit erstellen.
pub fn create_jwt_with_duration(
    user: &AuthUser,
    secret: &str,
    must_change_pw: bool,
    duration: chrono::Duration,
) -> Result<String, ApiError> {
    let exp = Utc::now() + duration;
    let claims = Claims {
        sub: user.id.clone(),
        username: user.username.clone(),
        display_name: user.display_name.clone(),
        role: user.role.clone(),
        exp: exp.timestamp() as usize,
        must_change_password: must_change_pw,
    };
    encode(
        &JwtHeader::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| ApiError::BadRequest(format!("JWT error: {e}")))
}

/// JWT-Token aus Cookie oder Authorization-Header extrahieren.
pub fn extract_token_from_headers(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|cookie_str| {
            cookie_str
                .split(';')
                .map(str::trim)
                .find_map(|c| c.strip_prefix("plankton_token="))
                .filter(|t| !t.is_empty())
                .map(String::from)
        })
        .or_else(|| {
            headers
                .get("authorization")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "))
                .map(String::from)
        })
}

/// JWT-Token validieren und Claims zurückgeben.
pub fn validate_jwt(token: &str, secret: &str) -> Result<Claims, ApiError> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|_| ApiError::Unauthorized("Invalid token".into()))
}

/// Serialisiert einen AuthUser ohne password_hash für API-Responses.
pub fn user_to_json(user: &AuthUser) -> serde_json::Value {
    serde_json::json!({
        "id": user.id,
        "username": user.username,
        "display_name": user.display_name,
        "role": user.role,
        "active": user.active,
        "created_at": user.created_at,
        "updated_at": user.updated_at,
    })
}
