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

/// Scope eines Agent-Tokens: global (für alle Nutzer) oder personal (nur Ersteller).
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum TokenScope {
    #[default]
    Global,
    Personal,
}

/// Ein Agent-Token für API-Zugriff ohne Login.
/// Das `token_hash` Feld enthält SHA-256-Hash des Secrets.
/// Das Klartextgeheimnis wird **nur beim Erstellen** zurückgegeben.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentToken {
    pub id: String,
    pub name: String,
    /// SHA-256-Hash des Token-Secrets (hex-kodiert). Kein Klartext gespeichert.
    pub token_hash: String,
    pub role: String,
    #[serde(default = "default_true")]
    pub active: bool,
    pub created_at: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub creator: String,
    #[serde(default)]
    pub last_used: Option<String>,
    #[serde(default)]
    pub scope: TokenScope,
    #[serde(default)]
    pub expires_at: Option<String>,
}

/// Anfrage zum Erstellen eines Agent-Tokens.
#[derive(Debug, Deserialize)]
pub struct CreateTokenRequest {
    pub name: String,
    pub role: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub scope: Option<TokenScope>,
    #[serde(default)]
    pub expires_at: Option<String>,
}

/// Anfrage zum Aktualisieren eines Agent-Tokens.
#[derive(Debug, Deserialize)]
pub struct UpdateTokenRequest {
    pub name: Option<String>,
    pub role: Option<String>,
    pub active: Option<bool>,
    pub description: Option<String>,
}

/// Hasht ein Token-Secret mit SHA-256 (hex-kodiert).
pub fn hash_token_secret(secret: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Verifiziert ein Token-Secret gegen seinen SHA-256-Hash.
pub fn verify_token_secret(secret: &str, hash: &str) -> bool {
    hash_token_secret(secret) == hash
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
    #[serde(default)]
    pub client_secret: String,
    pub name: String,
    pub redirect_uris: Vec<String>,
    /// "none" (PKCE public client) oder "client_secret_post"
    #[serde(default = "default_auth_method")]
    pub auth_method: String,
    #[serde(default = "default_true")]
    pub active: bool,
    pub created_at: String,
}

fn default_auth_method() -> String {
    "client_secret_post".to_string()
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
#[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;

    // --- Failing tests: müssen zunächst rot sein ---

    #[test]
    fn test_hash_token_secret_produces_hex_sha256() {
        // SHA-256("plk_test") soll ein 64-Zeichen langer Hex-String sein
        let secret = "plk_test";
        let hash = hash_token_secret(secret);
        assert_eq!(hash.len(), 64, "SHA-256 hex muss 64 Zeichen lang sein");
        assert!(
            hash.chars().all(|c| c.is_ascii_hexdigit()),
            "Nur Hex-Zeichen erwartet"
        );
    }

    #[test]
    fn test_verify_token_secret_correct() {
        let secret = generate_agent_token();
        let hash = hash_token_secret(&secret);
        assert!(
            verify_token_secret(&secret, &hash),
            "Korrekt: Secret passt zum Hash"
        );
    }

    #[test]
    fn test_verify_token_secret_wrong() {
        let hash = hash_token_secret("plk_correct");
        assert!(
            !verify_token_secret("plk_wrong", &hash),
            "Falsch: anderes Secret darf nicht passen"
        );
    }

    #[test]
    fn test_agent_token_no_plaintext_in_struct() {
        // AgentToken soll token_hash haben, kein token (Klartext)
        let token = AgentToken {
            id: "id1".to_string(),
            name: "test".to_string(),
            token_hash: hash_token_secret("plk_secret"),
            role: "user".to_string(),
            active: true,
            created_at: "2026-01-01".to_string(),
            description: "Beschreibung".to_string(),
            creator: "admin".to_string(),
            last_used: None,
            scope: TokenScope::Global,
            expires_at: None,
        };
        // JSON soll kein "token" Klartext-Feld enthalten, nur token_hash
        let json = serde_json::to_string(&token).unwrap();
        assert!(json.contains("token_hash"), "token_hash muss im JSON sein");
        assert!(
            !json.contains("\"token\":"),
            "Kein Klartext-token Feld im JSON"
        );
    }

    #[test]
    fn test_token_scope_default_is_global() {
        let scope = TokenScope::default();
        assert_eq!(scope, TokenScope::Global);
    }

    #[test]
    fn test_hash_is_deterministic() {
        // Gleicher Input → gleicher Hash (SHA-256 ist deterministisch)
        let secret = "plk_deterministic";
        assert_eq!(hash_token_secret(secret), hash_token_secret(secret));
    }

    #[test]
    fn test_different_secrets_different_hashes() {
        let h1 = hash_token_secret("plk_aaa");
        let h2 = hash_token_secret("plk_bbb");
        assert_ne!(
            h1, h2,
            "Verschiedene Secrets müssen verschiedene Hashes ergeben"
        );
    }

    #[test]
    fn test_create_token_request_has_description_and_scope() {
        // CreateTokenRequest soll description und scope akzeptieren
        let json = r#"{"name":"CI","role":"user","description":"For CI/CD","scope":"global"}"#;
        let req: CreateTokenRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "CI");
        assert_eq!(req.description, "For CI/CD");
        assert_eq!(req.scope, Some(TokenScope::Global));
    }

    #[test]
    fn test_create_token_request_minimal() {
        // Ohne description/scope soll es defaulten
        let json = r#"{"name":"CI","role":"user"}"#;
        let req: CreateTokenRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.description, "");
        assert!(req.scope.is_none());
    }
}
