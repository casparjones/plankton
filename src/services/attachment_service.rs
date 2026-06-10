// Abstraktionsschicht für Datei-Uploads: S3-Impl für Produktion, Memory-Impl für Tests.

use std::{collections::HashMap, sync::Arc, time::Duration};

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::error::ApiError;

/// Abstraktes Interface für Datei-Speicherung.
/// Ermöglicht austauschbare Backends (S3, lokal, Memory für Tests).
#[async_trait]
pub trait AttachmentStore: Send + Sync {
    /// Datei hochladen. Gibt die öffentliche oder presigned URL zurück.
    async fn upload(&self, key: &str, data: Vec<u8>, mime_type: &str) -> Result<String, ApiError>;
    /// Datei löschen.
    async fn delete(&self, key: &str) -> Result<(), ApiError>;
    /// Download-URL für eine Datei erzeugen (Presigned URL, TTL in Sekunden).
    async fn download_url(&self, key: &str, ttl_secs: u64) -> Result<String, ApiError>;
}

// ─────────────────────────────────────────────────────────
// S3AttachmentStore
// ─────────────────────────────────────────────────────────

pub struct S3AttachmentStore {
    pub client: aws_sdk_s3::Client,
    pub bucket: String,
    /// Optionale feste Basis-URL (z.B. CDN). Wenn gesetzt, wird sie statt Presigned URLs verwendet.
    pub public_url: Option<String>,
}

#[async_trait]
impl AttachmentStore for S3AttachmentStore {
    async fn upload(&self, key: &str, data: Vec<u8>, mime_type: &str) -> Result<String, ApiError> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(aws_sdk_s3::primitives::ByteStream::from(data))
            .content_type(mime_type)
            .send()
            .await
            .map_err(|e| ApiError::InternalError(format!("S3 upload failed: {e:?}")))?;

        // URL bauen: entweder feste public_url oder Presigned URL
        if let Some(base) = &self.public_url {
            Ok(format!("{}/{}", base.trim_end_matches('/'), key))
        } else {
            self.download_url(key, 3600).await
        }
    }

    async fn delete(&self, key: &str) -> Result<(), ApiError> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| ApiError::InternalError(format!("S3 delete failed: {e:?}")))?;
        Ok(())
    }

    async fn download_url(&self, key: &str, ttl_secs: u64) -> Result<String, ApiError> {
        let presigning =
            aws_sdk_s3::presigning::PresigningConfig::expires_in(Duration::from_secs(ttl_secs))
                .map_err(|e| ApiError::InternalError(format!("PresigningConfig error: {e}")))?;

        let presigned = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .presigned(presigning)
            .await
            .map_err(|e| ApiError::InternalError(format!("S3 presign failed: {e}")))?;

        Ok(presigned.uri().to_string())
    }
}

// ─────────────────────────────────────────────────────────
// MemoryAttachmentStore – nur für Tests
// ─────────────────────────────────────────────────────────

pub struct MemoryAttachmentStore {
    pub files: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    pub base_url: String,
}

#[allow(dead_code)]
impl MemoryAttachmentStore {
    pub fn new() -> Self {
        Self {
            files: Arc::new(Mutex::new(HashMap::new())),
            base_url: "http://memory-store".to_string(),
        }
    }
}

#[async_trait]
impl AttachmentStore for MemoryAttachmentStore {
    async fn upload(&self, key: &str, data: Vec<u8>, _mime_type: &str) -> Result<String, ApiError> {
        self.files.lock().await.insert(key.to_string(), data);
        Ok(format!("{}/{}", self.base_url, key))
    }

    async fn delete(&self, key: &str) -> Result<(), ApiError> {
        self.files.lock().await.remove(key);
        Ok(())
    }

    async fn download_url(&self, key: &str, _ttl_secs: u64) -> Result<String, ApiError> {
        Ok(format!("{}/{}", self.base_url, key))
    }
}

// ─────────────────────────────────────────────────────────
// S3-Client-Builder aus Config
// ─────────────────────────────────────────────────────────

/// Baut einen S3AttachmentStore aus der S3Config.
pub fn build_s3_store(cfg: &crate::config::S3Config) -> S3AttachmentStore {
    use aws_credential_types::Credentials;
    use aws_sdk_s3::config::Region;

    let creds = Credentials::new(&cfg.access_key, &cfg.secret_key, None, None, "plankton-s3");

    let s3_config = aws_sdk_s3::Config::builder()
        .endpoint_url(&cfg.endpoint)
        .credentials_provider(creds)
        .region(Region::new(cfg.region.clone()))
        // Path-style für S3-kompatible Stores wie MinIO.
        .force_path_style(true)
        .behavior_version_latest()
        .build();

    let client = aws_sdk_s3::Client::from_conf(s3_config);

    S3AttachmentStore {
        client,
        bucket: cfg.bucket.clone(),
        public_url: cfg.public_url.clone(),
    }
}
