// Zentraler Anwendungs-State.

use std::{collections::HashMap, sync::Arc};
use tokio::sync::{broadcast, Mutex};

use crate::models::CliSession;
use crate::store::DataStore;

/// MCP-Session für Streamable HTTP Transport.
pub struct McpSession {
    pub caller: String,
    pub role: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub tx: broadcast::Sender<String>,
}

/// Zentraler Anwendungs-State, der von Axum in alle Handler injiziert wird.
#[derive(Clone)]
pub struct AppState {
    pub store: DataStore,
    pub events: Arc<Mutex<HashMap<String, broadcast::Sender<String>>>>,
    pub jwt_secret: String,
    pub cli_sessions: Arc<Mutex<HashMap<String, CliSession>>>,
    pub mcp_sessions: Arc<Mutex<HashMap<String, McpSession>>>,
}
