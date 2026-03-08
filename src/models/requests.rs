// Request/Response-Hilfstypen für API-Endpunkte und MCP.

use serde::{Deserialize, Serialize};

/// Query-Parameter für DELETE-Anfragen mit CouchDB-Revision.
#[derive(Debug, Deserialize)]
pub struct DeleteQuery {
    pub rev: String,
}

/// Query-Parameter für GET /projects/:id – optionales Archiv-Flag.
#[derive(Debug, Deserialize)]
pub struct GetProjectQuery {
    #[serde(default)]
    pub include_archived: bool,
}

/// Body für POST /projects/:id/tasks/:task_id/move
#[derive(Debug, Deserialize)]
pub struct MoveTaskRequest {
    pub column_id: String,
    pub order: Option<i32>,
}

/// Body für POST /mcp/call
#[derive(Debug, Deserialize)]
pub struct McpCall {
    pub tool: String,
    pub arguments: serde_json::Value,
}

/// Tool-Beschreibung für GET /mcp/tools
#[derive(Debug, Serialize, Clone)]
pub struct ToolDef {
    pub name: &'static str,
    pub description: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<&'static [&'static str]>,
}

/// JSON-RPC 2.0 Request
#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: Option<String>,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
    pub id: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 Response
#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    pub id: serde_json::Value,
}

/// JSON-RPC 2.0 Fehler-Objekt.
#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

/// Body für POST /projects/:id/import
#[derive(Debug, Deserialize)]
pub struct ImportRequest {
    pub tasks: Vec<super::project::Task>,
}

/// Response für POST /projects/:id/import
#[derive(Debug, Serialize)]
pub struct ImportResponse {
    pub imported: usize,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub skipped: usize,
}
