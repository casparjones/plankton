// Konfiguration aus Umgebungsvariablen.

/// Anwendungskonfiguration, gelesen aus Umgebungsvariablen.
pub struct Config {
    pub couch_uri: Option<String>,
    pub db: String,
    pub port: String,
    pub jwt_secret: Option<String>,
}

impl Config {
    /// Liest die Konfiguration aus den Umgebungsvariablen.
    pub fn from_env() -> Self {
        let couch_uri = std::env::var("COUCHDB_URI")
            .ok()
            .or_else(|| std::env::var("COUCHDB_URL").ok());
        let db = std::env::var("COUCHDB_DB").unwrap_or_else(|_| "plankton".to_string());
        let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
        let jwt_secret = std::env::var("JWT_SECRET").ok();

        Self {
            couch_uri,
            db,
            port,
            jwt_secret,
        }
    }
}
