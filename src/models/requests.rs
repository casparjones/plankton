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

/// Einzelner Move in einem Batch.
#[derive(Debug, Deserialize)]
pub struct BatchMoveItem {
    pub task_id: String,
    pub column_id: String,
    pub order: i32,
}

/// Body für POST /projects/:id/tasks/batch-move
#[derive(Debug, Deserialize)]
pub struct BatchMoveRequest {
    pub moves: Vec<BatchMoveItem>,
}

/// Body für PUT /projects/:id/tasks/:task_id – Partielles Update.
#[derive(Debug, Deserialize)]
pub struct UpdateTaskRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub column_id: Option<String>,
    pub labels: Option<Vec<String>>,
    pub worker: Option<String>,
    pub points: Option<i32>,
    pub order: Option<i32>,
    pub comments: Option<Vec<serde_json::Value>>,
    pub logs: Option<Vec<serde_json::Value>>,
    pub task_type: Option<String>,
    pub blocks: Option<Vec<String>>,
    pub blocked_by: Option<Vec<String>>,
    pub parent_id: Option<String>,
    pub subtask_ids: Option<Vec<String>>,
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
