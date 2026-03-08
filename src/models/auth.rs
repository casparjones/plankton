// Datenmodelle für Authentifizierung, JWT und Agent-Tokens.

use serde::{Deserialize, Serialize};

/// Hilfsfunktion für serde-Default: gibt `true` zurück.
pub fn default_true() -> bool {
    true
}

/// Ein registrierter Benutzer mit Passwort-Hash und Rolle.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthUser {
    pub id: String,
    pub username: String,
    pub display_name: String,
    #[serde(default)]
    pub password_hash: String,
    pub role: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default = "default_true")]
    pub active: bool,
}

/// JWT-Claims für die Token-Validierung.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub username: String,
    pub display_name: String,
    pub role: String,
    pub exp: usize,
    #[serde(default)]
    pub must_change_password: bool,
}

/// Login-Anfrage: Benutzername und Passwort.
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Passwort-Änderungs-Anfrage.
#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

/// Admin: Neuen Benutzer anlegen.
#[derive(Debug, Deserialize)]
pub struct CreateAuthUserRequest {
    pub username: String,
    pub display_name: String,
    pub password: String,
    pub role: String,
}

/// Admin: Benutzer aktualisieren.
#[derive(Debug, Deserialize)]
pub struct UpdateAuthUserRequest {
    pub display_name: Option<String>,
    pub role: Option<String>,
    pub active: Option<bool>,
}

/// Admin: Passwort zurücksetzen.
#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub password: String,
}

/// Ein Agent-Token für API-Zugriff ohne Login.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentToken {
    pub id: String,
    pub name: String,
    pub token: String,
    pub role: String,
    #[serde(default = "default_true")]
    pub active: bool,
    pub created_at: String,
}

/// Anfrage zum Erstellen eines Agent-Tokens.
#[derive(Debug, Deserialize)]
pub struct CreateTokenRequest {
    pub name: String,
    pub role: String,
}

/// Anfrage zum Aktualisieren eines Agent-Tokens.
#[derive(Debug, Deserialize)]
pub struct UpdateTokenRequest {
    pub name: Option<String>,
    pub role: Option<String>,
    pub active: Option<bool>,
}

/// Generiert einen zufälligen Agent-Token im Format `plk_<hex>`.
pub fn generate_agent_token() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 24] = rng.gen();
    let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
    format!("plk_{}", hex)
}
