// Zentraler Anwendungs-State.

use std::{collections::HashMap, sync::Arc};
use tokio::sync::{broadcast, Mutex};

use crate::models::{CliSession, OAuthAuthCode, OAuthClient, OAuthRefreshToken};
use crate::store::DataStore;
use reqwest::Client;

/// MCP-Session für Streamable HTTP Transport.
#[allow(dead_code)]
pub struct McpSession {
    pub caller: String,
    pub role: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub tx: broadcast::Sender<String>,
}

/// Zentraler Anwendungs-State, der von Axum in alle Handler injiziert wird.
#[derive(Clone)]
#[allow(dead_code)]
pub struct AppState {
    pub store: DataStore,
    pub events: Arc<Mutex<HashMap<String, broadcast::Sender<String>>>>,
    pub jwt_secret: String,
    pub cli_sessions: Arc<Mutex<HashMap<String, CliSession>>>,
    pub mcp_sessions: Arc<Mutex<HashMap<String, McpSession>>>,
    pub oauth_clients: Arc<Mutex<Vec<OAuthClient>>>,
    pub oauth_codes: Arc<Mutex<HashMap<String, OAuthAuthCode>>>,
    pub oauth_refresh_tokens: Arc<Mutex<HashMap<String, OAuthRefreshToken>>>,
    /// Layer 1: Per-Projekt Write-Locks für serialisierte Schreibzugriffe.
    /// Verhindert Race Conditions und Datei-Korruption bei parallelen Agenten.
    pub write_locks: Arc<Mutex<HashMap<String, Arc<Mutex<()>>>>>,
    /// HTTP-Client für ausgehende Webhooks (wiederverwendet Connection-Pool).
    pub http_client: Client,
}

impl AppState {
    /// Gibt den projektspezifischen Write-Lock zurück (lazy initialized).
    /// Alle Schreiboperationen auf dasselbe Projekt werden serialisiert.
    pub async fn get_project_write_lock(&self, project_id: &str) -> Arc<Mutex<()>> {
        let mut locks = self.write_locks.lock().await;
        locks
            .entry(project_id.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }
}
