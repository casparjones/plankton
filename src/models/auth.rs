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

/// Status einer CLI-Login-Session (Device Flow).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CliSessionStatus {
    Pending,
    Approved,
    Expired,
}

/// Eine CLI-Login-Session für den Device-Auth-Flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliSession {
    pub session_id: String,
    pub code: String,
    pub status: CliSessionStatus,
    pub token: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Anfrage zum Genehmigen einer CLI-Session.
#[derive(Debug, Deserialize)]
pub struct CliApproveRequest {
    pub session_id: String,
}

/// Generiert einen zufälligen Agent-Token im Format `plk_<hex>`.
pub fn generate_agent_token() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 24] = rng.gen();
    let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
    format!("plk_{}", hex)
}

// ─── OAuth 2.0 ──────────────────────────────────────────────

/// Registrierter OAuth 2.0 Client.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OAuthClient {
    pub client_id: String,
    pub client_secret: String,
    pub name: String,
    pub redirect_uris: Vec<String>,
    #[serde(default = "default_true")]
    pub active: bool,
    pub created_at: String,
}

/// OAuth 2.0 Authorization Code (kurzlebig, einmalig einlösbar).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OAuthAuthCode {
    pub code: String,
    pub client_id: String,
    pub user_id: String,
    pub redirect_uri: String,
    pub scope: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// PKCE code_challenge (S256)
    #[serde(default)]
    pub code_challenge: Option<String>,
}

/// OAuth 2.0 Refresh Token.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OAuthRefreshToken {
    pub token: String,
    pub client_id: String,
    pub user_id: String,
    pub scope: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(default = "default_true")]
    pub active: bool,
}

/// OAuth 2.0 Authorization Request (Query-Parameter).
#[derive(Debug, Deserialize)]
pub struct OAuthAuthorizeRequest {
    pub response_type: String,
    pub client_id: String,
    pub redirect_uri: String,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub code_challenge: Option<String>,
    #[serde(default)]
    pub code_challenge_method: Option<String>,
}

/// OAuth 2.0 Token Request (POST body).
#[derive(Debug, Deserialize)]
pub struct OAuthTokenRequest {
    pub grant_type: String,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub redirect_uri: Option<String>,
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub client_secret: Option<String>,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub code_verifier: Option<String>,
}

/// Generiert einen zufälligen OAuth-Code oder Token.
pub fn generate_oauth_code() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen();
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
