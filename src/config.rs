// Konfiguration aus Umgebungsvariablen.

/// S3-Konfiguration für File-Attachments.
/// Nur aktiv wenn S3_BUCKET gesetzt ist – sonst ist der Upload-Feature deaktiviert.
#[derive(Clone, Debug)]
pub struct S3Config {
    /// S3-kompatibler Endpunkt (z.B. https://s3.amazonaws.com oder https://minio.example.com).
    pub endpoint: String,
    /// Bucket-Name.
    pub bucket: String,
    /// Access Key ID.
    pub access_key: String,
    /// Secret Access Key.
    pub secret_key: String,
    /// Region (default: us-east-1).
    pub region: String,
    /// Optionale öffentliche Basis-URL für Links (z.B. CDN-URL).
    /// Wenn nicht gesetzt, werden Presigned S3 URLs verwendet.
    pub public_url: Option<String>,
}

/// Anwendungskonfiguration, gelesen aus Umgebungsvariablen.
pub struct Config {
    pub couch_uri: Option<String>,
    pub db: String,
    pub port: String,
    pub jwt_secret: Option<String>,
    pub s3: Option<S3Config>,
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

        // S3 nur aktivieren wenn S3_BUCKET gesetzt ist.
        let s3 = std::env::var("S3_BUCKET").ok().and_then(|bucket| {
            let endpoint = std::env::var("S3_ENDPOINT")
                .unwrap_or_else(|_| "https://s3.amazonaws.com".to_string());
            let access_key = std::env::var("S3_ACCESS_KEY").ok()?;
            let secret_key = std::env::var("S3_SECRET_KEY").ok()?;
            let region = std::env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".to_string());
            let public_url = std::env::var("S3_PUBLIC_URL").ok();
            Some(S3Config {
                endpoint,
                bucket,
                access_key,
                secret_key,
                region,
                public_url,
            })
        });

        Self {
            couch_uri,
            db,
            port,
            jwt_secret,
            s3,
        }
    }
}
