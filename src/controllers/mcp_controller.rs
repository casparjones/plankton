// Handler für MCP-Endpunkte (Legacy + Streamable HTTP Transport) und Docs.

use axum::{
    extract::State,
    response::{sse::Event, Sse},
    Json,
};
use chrono::Utc;
use futures::stream;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::error::ApiError;
use crate::models::*;
use crate::services::*;
use crate::state::{AppState, McpSession};

/// Alle verfügbaren MCP-Tools mit optionaler Rollen-Einschränkung.
fn all_tools() -> Vec<ToolDef> {
    vec![
        ToolDef { name: "list_projects", description: "List all projects with id, title, slug, task_count. Use this first to find project IDs.", roles: None, schema: None },
        ToolDef { name: "get_project", description: "Get project with columns and tasks (compact: no logs/comments). Use get_task for full task details.", roles: None, schema: Some(|| serde_json::json!({
            "type": "object", "required": ["id"],
            "properties": { "id": { "type": "string", "description": "Project ID" } }
        })) },
        ToolDef { name: "get_task", description: "Get full task details including description, comments, logs. Use after get_project to see a specific task.", roles: None, schema: Some(|| serde_json::json!({
            "type": "object", "required": ["project_id", "task_id"],
            "properties": { "project_id": { "type": "string" }, "task_id": { "type": "string" } }
        })) },
        ToolDef { name: "summarize_board", description: "Quick overview: column names with task counts per column", roles: None, schema: Some(|| serde_json::json!({
            "type": "object", "required": ["project_id"],
            "properties": { "project_id": { "type": "string", "description": "Project ID" } }
        })) },
        ToolDef { name: "create_project", description: "Create a new project", roles: Some(&["manager", "admin"]), schema: Some(|| serde_json::json!({
            "type": "object",
            "properties": { "title": { "type": "string", "description": "Project title" } }
        })) },
        ToolDef { name: "update_project", description: "Update project metadata (title, owner, type, doneExpire, archiveDelete, pinned)", roles: Some(&["manager", "admin"]), schema: Some(|| serde_json::json!({
            "type": "object", "required": ["project_id"],
            "properties": {
                "project_id": { "type": "string", "description": "Project ID or slug" },
                "title": { "type": "string", "description": "New project title" },
                "owner": { "type": "string", "description": "Project owner (username/display_name)" },
                "type": { "type": "string", "enum": ["kanban", "list"], "description": "Board type: 'kanban' (default) or 'list'" },
                "done_expire": { "type": "integer", "description": "Days until Done tasks are archived. Default: 10. -1 = disabled." },
                "archive_delete": { "type": "integer", "description": "Days until archived tasks are deleted. Default: 90. -1 = disabled." },
                "pinned": { "type": "boolean", "description": "Pin this board at the top of the Move-to-Board selector." }
            }
        })) },
        ToolDef { name: "list_epics", description: "List columns as epics with task counts", roles: Some(&["manager", "admin"]), schema: Some(|| serde_json::json!({
            "type": "object", "required": ["project_id"],
            "properties": { "project_id": { "type": "string" } }
        })) },
        ToolDef { name: "create_task", description: "Create a new task. Returns the created task object. Tasks land in the first column (Todo) by default.", roles: Some(&["manager", "admin"]), schema: Some(|| serde_json::json!({
            "type": "object", "required": ["project_id"],
            "properties": {
                "project_id": { "type": "string" },
                "title": { "type": "string" },
                "description": { "type": "string" },
                "column_id": { "type": "string", "description": "Column ID (default: first column)" },
                "labels": { "type": "array", "items": { "type": "string" } },
                "worker": { "type": "string" },
                "points": { "type": "number" },
                "task_type": { "type": "string", "enum": ["task", "epic", "job"] },
                "parent_id": { "type": "string", "description": "Parent epic ID for subtasks" }
            }
        })) },
        ToolDef { name: "assign_task", description: "Assign a worker to a task. Pass _rev (from get_task/get_project) to enable optimistic locking (409 Conflict if stale).", roles: Some(&["manager", "admin"]), schema: Some(|| serde_json::json!({
            "type": "object", "required": ["project_id", "task_id", "worker"],
            "properties": { "project_id": { "type": "string" }, "task_id": { "type": "string" }, "worker": { "type": "string" }, "_rev": { "type": "string", "description": "Optional project revision for optimistic locking" } }
        })) },
        ToolDef { name: "get_assigned_tasks", description: "Get tasks assigned to the caller", roles: Some(&["developer"]), schema: Some(|| serde_json::json!({
            "type": "object", "required": ["project_id"],
            "properties": { "project_id": { "type": "string" } }
        })) },
        ToolDef { name: "update_task", description: "Update task title/description/labels. Pass _rev (from get_task/get_project) to enable optimistic locking (409 Conflict if stale).", roles: Some(&["developer", "manager", "admin"]), schema: Some(|| serde_json::json!({
            "type": "object", "required": ["project_id", "task_id"],
            "properties": {
                "project_id": { "type": "string" }, "task_id": { "type": "string" },
                "title": { "type": "string" }, "description": { "type": "string" },
                "labels": { "type": "array", "items": { "type": "string" } },
                "worker": { "type": "string" }, "points": { "type": "number" },
                "task_type": { "type": "string", "enum": ["task", "epic", "job"] },
                "parent_id": { "type": "string" },
                "_rev": { "type": "string", "description": "Optional project revision for optimistic locking" }
            }
        })) },
        ToolDef { name: "add_log", description: "DEPRECATED: Use add_comment instead. Kept for backward compatibility — internally routes to add_comment.", roles: Some(&["developer", "tester", "manager", "admin"]), schema: Some(|| serde_json::json!({
            "type": "object", "required": ["project_id", "task_id", "message"],
            "properties": { "project_id": { "type": "string" }, "task_id": { "type": "string" }, "message": { "type": "string" } }
        })) },
        ToolDef { name: "submit_for_review", description: "Mark task as ready for review", roles: Some(&["developer"]), schema: Some(|| serde_json::json!({
            "type": "object", "required": ["project_id", "task_id"],
            "properties": { "project_id": { "type": "string" }, "task_id": { "type": "string" } }
        })) },
        ToolDef { name: "get_review_queue", description: "Get tasks waiting for review", roles: Some(&["tester"]), schema: Some(|| serde_json::json!({
            "type": "object", "required": ["project_id"],
            "properties": { "project_id": { "type": "string" } }
        })) },
        ToolDef { name: "add_comment", description: "Add a comment to a task. This is the primary tool for agent communication: validation results, decisions, review feedback, handoffs between agents. Prefer add_comment over add_log.", roles: Some(&["tester", "developer", "manager", "admin"]), schema: Some(|| serde_json::json!({
            "type": "object", "required": ["project_id", "task_id", "text"],
            "properties": { "project_id": { "type": "string" }, "task_id": { "type": "string" }, "text": { "type": "string" } }
        })) },
        ToolDef { name: "approve_task", description: "Approve and move task to Done", roles: Some(&["tester", "manager", "admin"]), schema: Some(|| serde_json::json!({
            "type": "object", "required": ["project_id", "task_id"],
            "properties": { "project_id": { "type": "string" }, "task_id": { "type": "string" } }
        })) },
        ToolDef { name: "reject_task", description: "Reject task and move back with comment", roles: Some(&["tester", "manager", "admin"]), schema: Some(|| serde_json::json!({
            "type": "object", "required": ["project_id", "task_id"],
            "properties": { "project_id": { "type": "string" }, "task_id": { "type": "string" }, "comment": { "type": "string" } }
        })) },
        ToolDef { name: "move_task", description: "Move a task between columns. Pass _rev (from get_task/get_project) to enable optimistic locking (409 Conflict if stale). BLOCKING: Moving a task to 'In Progress' fails with 400 if any blocker in blocked_by is not yet in 'Done'. Use add_relation(relation='blocks') to set dependencies.", roles: Some(&["manager", "admin"]), schema: Some(|| serde_json::json!({
            "type": "object", "required": ["project_id", "task_id", "column_id"],
            "properties": { "project_id": { "type": "string" }, "task_id": { "type": "string" }, "column_id": { "type": "string" }, "order": { "type": "number" }, "_rev": { "type": "string", "description": "Optional project revision for optimistic locking" } }
        })) },
        ToolDef { name: "delete_task", description: "Delete a task. Pass _rev (from get_task/get_project) to enable optimistic locking (409 Conflict if stale).", roles: Some(&["manager", "admin"]), schema: Some(|| serde_json::json!({
            "type": "object", "required": ["project_id", "task_id"],
            "properties": { "project_id": { "type": "string" }, "task_id": { "type": "string" }, "_rev": { "type": "string", "description": "Optional project revision for optimistic locking" } }
        })) },
        ToolDef { name: "list_subtasks", description: "List subtasks of an epic with completion status", roles: None, schema: Some(|| serde_json::json!({
            "type": "object", "required": ["project_id", "parent_id"],
            "properties": { "project_id": { "type": "string" }, "parent_id": { "type": "string" } }
        })) },
        ToolDef { name: "add_relation", description: "Add a relation (blocks or subtask) between two tasks", roles: Some(&["developer", "manager", "admin"]), schema: Some(|| serde_json::json!({
            "type": "object", "required": ["project_id", "from_task_id", "to_task_id", "relation"],
            "properties": { "project_id": { "type": "string" }, "from_task_id": { "type": "string" }, "to_task_id": { "type": "string" }, "relation": { "type": "string", "enum": ["blocks", "subtask"] } }
        })) },
        ToolDef { name: "remove_relation", description: "Remove a relation between two tasks", roles: Some(&["developer", "manager", "admin"]), schema: Some(|| serde_json::json!({
            "type": "object", "required": ["project_id", "from_task_id", "to_task_id", "relation"],
            "properties": { "project_id": { "type": "string" }, "from_task_id": { "type": "string" }, "to_task_id": { "type": "string" }, "relation": { "type": "string", "enum": ["blocks", "subtask"] } }
        })) },
        ToolDef { name: "reorder_tasks", description: "Reorder tasks within a column by providing task IDs in desired order", roles: Some(&["manager", "admin"]), schema: Some(|| serde_json::json!({
            "type": "object", "required": ["project_id", "column_id", "task_ids"],
            "properties": { "project_id": { "type": "string" }, "column_id": { "type": "string" }, "task_ids": { "type": "array", "items": { "type": "string" } } }
        })) },
        ToolDef { name: "create_task_from_template", description: "Create a task from a named template. Built-in templates: bug, feature, security, epic, chore. Custom templates can be placed in .plankton/templates/<name>.json. Supports {{title}} and {{date}} variable substitution.", roles: Some(&["developer", "manager", "admin"]), schema: Some(|| serde_json::json!({
            "type": "object", "required": ["project_id", "template_name"],
            "properties": {
                "project_id": { "type": "string", "description": "Project ID" },
                "template_name": { "type": "string", "description": "Template name: 'bug', 'feature', 'security', 'epic', 'chore', or custom name matching .plankton/templates/<name>.json" },
                "title": { "type": "string", "description": "Task title (replaces {{title}} in template)" },
                "labels": { "type": "array", "items": { "type": "string" }, "description": "Additional labels (merged with template labels)" },
                "worker": { "type": "string", "description": "Assigned worker" },
                "points": { "type": "number", "description": "Story points" },
                "column_id": { "type": "string", "description": "Target column (default: first column)" },
                "parent_id": { "type": "string", "description": "Parent epic ID for subtasks" }
            }
        })) },
        ToolDef { name: "move_task_to_project", description: "Move a task from one project to another. Column mapping: the task lands in the column with the same name in the target project, or in the first column (order=0) if no match. Relations, comments, and logs are preserved. Returns the new task_id and column_id in the target project.", roles: Some(&["manager", "admin"]), schema: Some(|| serde_json::json!({
            "type": "object", "required": ["task_id", "source_project_id", "target_project_id"],
            "properties": {
                "task_id": { "type": "string", "description": "ID of the task to move" },
                "source_project_id": { "type": "string", "description": "ID of the source project" },
                "target_project_id": { "type": "string", "description": "ID of the target project (must be different from source)" }
            }
        })) },
        ToolDef { name: "attach_file", description: "Attach a small file (≤500 KB) to a task via base64-encoded content. Ideal for source code, configs, small PDFs. For larger files use the CLI: `plankton attach`. Only available when S3 is configured.", roles: Some(&["developer", "manager", "admin"]), schema: Some(|| serde_json::json!({
            "type": "object", "required": ["project_id", "task_id", "filename", "content_base64"],
            "properties": {
                "project_id": { "type": "string" },
                "task_id": { "type": "string" },
                "filename": { "type": "string", "description": "Filename including extension, e.g. main.rs" },
                "content_base64": { "type": "string", "description": "File content base64-encoded (standard encoding). Max 500 KB decoded." },
                "mime_type": { "type": "string", "description": "MIME type (optional, auto-detected from filename if omitted)" }
            }
        })) },
        ToolDef { name: "list_attachments", description: "List all file attachments of a task.", roles: None, schema: Some(|| serde_json::json!({
            "type": "object", "required": ["project_id", "task_id"],
            "properties": {
                "project_id": { "type": "string" },
                "task_id": { "type": "string" }
            }
        })) },
        ToolDef { name: "get_attachment", description: "Get download URL (presigned S3, valid 1h) for a task attachment.", roles: None, schema: Some(|| serde_json::json!({
            "type": "object", "required": ["project_id", "task_id", "attachment_id"],
            "properties": {
                "project_id": { "type": "string" },
                "task_id": { "type": "string" },
                "attachment_id": { "type": "string" }
            }
        })) },
        ToolDef { name: "delete_attachment", description: "Delete a file attachment from a task and S3.", roles: Some(&["developer", "manager", "admin"]), schema: Some(|| serde_json::json!({
            "type": "object", "required": ["project_id", "task_id", "attachment_id"],
            "properties": {
                "project_id": { "type": "string" },
                "task_id": { "type": "string" },
                "attachment_id": { "type": "string" }
            }
        })) },
    ]
}

/// Tools nach Rolle filtern. Admin und User sehen alle Tools.
fn tools_for_role(role: &str) -> Vec<ToolDef> {
    // Admin und User (OAuth-Login) sehen alle Tools
    if role == "admin" || role == "user" {
        return all_tools();
    }
    all_tools()
        .into_iter()
        .filter(|t| match t.roles {
            None => true,
            Some(roles) => roles.contains(&role),
        })
        .collect()
}

/// Caller-Identität aus Headers auflösen (JWT oder Agent-Token).
/// Gibt Err(Unauthorized) zurück wenn kein gültiger Token gefunden wird.
async fn resolve_caller(
    headers: &axum::http::HeaderMap,
    state: &AppState,
) -> Result<(String, String), ApiError> {
    if let Some(t) = extract_token_from_headers(headers) {
        if let Ok(claims) = validate_jwt(&t, &state.jwt_secret) {
            return Ok((claims.display_name, claims.role));
        }
    }
    if let Some(bearer) = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
    {
        if let Ok(agent_token) = state.store.get_token_by_value(bearer).await {
            return Ok((agent_token.name, agent_token.role));
        }
    }
    Err(ApiError::Unauthorized("Invalid or missing token".into()))
}

/// GET /mcp/tools – Verfügbare Tools auflisten.
pub async fn list_tools(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Json<Vec<ToolDef>> {
    match resolve_caller(&headers, &state).await {
        Ok((_, role)) if !role.is_empty() => Json(tools_for_role(&role)),
        _ => Json(all_tools()),
    }
}

/// POST /mcp/call – Ein Tool aufrufen (Legacy-Endpunkt).
pub async fn call_tool(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(call): Json<McpCall>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let (caller, _) = resolve_caller(&headers, &state).await?;
    let out = execute_tool(&state, &call.tool, &call.arguments, &caller).await?;
    Ok(Json(out))
}

/// Erzeugt eine 401-Response mit WWW-Authenticate Header (RFC 9728 Protected Resource).
fn unauthorized_response(host: &str, scheme: &str) -> axum::response::Response {
    use axum::http::{header, StatusCode};
    use axum::response::IntoResponse;
    let resource_url = format!("{scheme}://{host}/.well-known/oauth-protected-resource");
    let www_auth = format!(
        "Bearer realm=\"OAuth\", resource_metadata=\"{resource_url}\", error=\"invalid_token\", error_description=\"Missing or invalid access token\""
    );
    (
        StatusCode::UNAUTHORIZED,
        [(header::WWW_AUTHENTICATE, www_auth)],
        axum::Json(serde_json::json!({
            "error": "invalid_token",
            "error_description": "Missing or invalid access token"
        })),
    )
        .into_response()
}

/// Prüft ob der Client SSE-Streaming akzeptiert (Accept: text/event-stream).
fn wants_sse(headers: &axum::http::HeaderMap) -> bool {
    headers
        .get("accept")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.contains("text/event-stream"))
        .unwrap_or(false)
}

/// Verarbeitet eine einzelne JSON-RPC-Anfrage und gibt die Antwort zurück.
async fn handle_single_rpc(
    state: &AppState,
    rpc: &JsonRpcRequest,
    caller: &str,
    caller_role: &str,
    _session_id: &Option<String>,
) -> Option<JsonRpcResponse> {
    let id = rpc.id.clone().unwrap_or(serde_json::Value::Null);
    // Notifications (kein id-Feld) erhalten keine Antwort
    let is_notification = rpc.id.is_none();

    let response = match rpc.method.as_str() {
        "initialize" => {
            // initialize wird separat behandelt (neue Session)
            // Dieser Pfad sollte nicht erreicht werden, da mcp_jsonrpc es vorher abfängt.
            JsonRpcResponse {
                jsonrpc: "2.0".into(),
                result: Some(serde_json::json!({"error": "initialize handled separately"})),
                error: None,
                id,
            }
        }
        "initialized" | "notifications/initialized" => {
            if is_notification {
                return None;
            }
            JsonRpcResponse {
                jsonrpc: "2.0".into(),
                result: Some(serde_json::json!({})),
                error: None,
                id,
            }
        }
        "notifications/cancelled" => {
            // Client hat eine Anfrage abgebrochen – acknowledged.
            return None;
        }
        "ping" => JsonRpcResponse {
            jsonrpc: "2.0".into(),
            result: Some(serde_json::json!({})),
            error: None,
            id,
        },
        "tools/list" => {
            let tools = if caller_role.is_empty() {
                all_tools()
            } else {
                tools_for_role(caller_role)
            };
            let tool_list: Vec<_> = tools
                .iter()
                .map(|t| {
                    let schema = t
                        .schema
                        .map(|f| f())
                        .unwrap_or_else(|| serde_json::json!({"type": "object", "properties": {}}));
                    serde_json::json!({
                        "name": t.name,
                        "description": t.description,
                        "inputSchema": schema
                    })
                })
                .collect();
            JsonRpcResponse {
                jsonrpc: "2.0".into(),
                result: Some(serde_json::json!({ "tools": tool_list })),
                error: None,
                id,
            }
        }
        "tools/call" => {
            let tool_name = rpc.params["name"].as_str().unwrap_or("");
            let arguments = rpc
                .params
                .get("arguments")
                .cloned()
                .unwrap_or(serde_json::json!({}));
            match execute_tool(state, tool_name, &arguments, caller).await {
                Ok(result) => JsonRpcResponse {
                    jsonrpc: "2.0".into(),
                    result: Some(serde_json::json!({
                        "content": [{ "type": "text", "text": serde_json::to_string_pretty(&result).unwrap_or_default() }]
                    })),
                    error: None,
                    id,
                },
                Err(e) => JsonRpcResponse {
                    jsonrpc: "2.0".into(),
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32000,
                        message: format!("{e:?}"),
                    }),
                    id,
                },
            }
        }
        "resources/list" => JsonRpcResponse {
            jsonrpc: "2.0".into(),
            result: Some(serde_json::json!({ "resources": [] })),
            error: None,
            id,
        },
        "prompts/list" => JsonRpcResponse {
            jsonrpc: "2.0".into(),
            result: Some(serde_json::json!({ "prompts": [] })),
            error: None,
            id,
        },
        _ => JsonRpcResponse {
            jsonrpc: "2.0".into(),
            result: None,
            error: Some(JsonRpcError {
                code: -32601,
                message: format!("Method not found: {}", rpc.method),
            }),
            id,
        },
    };

    if is_notification {
        None
    } else {
        Some(response)
    }
}

/// POST /mcp – JSON-RPC 2.0 MCP-Endpunkt mit Streamable HTTP Transport.
/// Unterstützt sowohl JSON- als auch SSE-Antworten (je nach Accept-Header).
/// Erstellt bei "initialize" eine Session und gibt Mcp-Session-Id Header zurück.
pub async fn mcp_jsonrpc(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> axum::response::Response {
    use axum::http::{header, StatusCode};
    use axum::response::IntoResponse;

    let use_sse = wants_sse(&headers);
    let auth_result = resolve_caller(&headers, &state).await;

    // Host/Scheme für WWW-Authenticate Header
    let scheme = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("http");
    let host = headers
        .get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("localhost");

    // Session-ID aus Header
    let session_id = headers
        .get("mcp-session-id")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    // Auth-Check VOR Body-Parsing: Kein Token UND keine gültige Session → 401
    if auth_result.is_err() {
        let has_valid_session = if let Some(ref sid) = session_id {
            let sessions = state.mcp_sessions.lock().await;
            sessions
                .get(sid)
                .map(|s| !s.caller.is_empty() && !s.role.is_empty())
                .unwrap_or(false)
        } else {
            false
        };
        if !has_valid_session {
            return unauthorized_response(host, scheme);
        }
    }

    // Versuche als einzelne Anfrage oder Batch zu parsen
    let raw: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            let resp = JsonRpcResponse {
                jsonrpc: "2.0".into(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32700,
                    message: format!("Parse error: {e}"),
                }),
                id: serde_json::Value::Null,
            };
            return Json(resp).into_response();
        }
    };

    // Batch oder einzeln?
    let requests: Vec<JsonRpcRequest> = if raw.is_array() {
        match serde_json::from_value(raw) {
            Ok(batch) => batch,
            Err(e) => {
                let resp = JsonRpcResponse {
                    jsonrpc: "2.0".into(),
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32600,
                        message: format!("Invalid batch: {e}"),
                    }),
                    id: serde_json::Value::Null,
                };
                return Json(resp).into_response();
            }
        }
    } else {
        match serde_json::from_value(raw) {
            Ok(single) => vec![single],
            Err(e) => {
                let resp = JsonRpcResponse {
                    jsonrpc: "2.0".into(),
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32600,
                        message: format!("Invalid request: {e}"),
                    }),
                    id: serde_json::Value::Null,
                };
                return Json(resp).into_response();
            }
        }
    };

    if requests.is_empty() {
        return StatusCode::BAD_REQUEST.into_response();
    }

    // Session-Validierung (außer für initialize)
    let has_init = requests.iter().any(|r| r.method == "initialize");
    let mut session_id = session_id;
    if !has_init {
        match &session_id {
            Some(sid) => {
                let sessions = state.mcp_sessions.lock().await;
                if !sessions.contains_key(sid) {
                    // Ungültige Session aber gültiger Token → neue Session erstellen
                    if let Ok((caller, role)) = &auth_result {
                        drop(sessions);
                        let new_sid = Uuid::new_v4().to_string();
                        let (tx, _) = broadcast::channel::<String>(100);
                        state.mcp_sessions.lock().await.insert(
                            new_sid.clone(),
                            McpSession {
                                caller: caller.clone(),
                                role: role.clone(),
                                created_at: Utc::now(),
                                tx,
                            },
                        );
                        session_id = Some(new_sid);
                    } else {
                        return unauthorized_response(host, scheme);
                    }
                }
            }
            None => {
                // Kein Session-Header: Mit gültigem Token → automatisch Session erstellen
                if let Ok((caller, role)) = &auth_result {
                    let new_sid = Uuid::new_v4().to_string();
                    let (tx, _) = broadcast::channel::<String>(100);
                    state.mcp_sessions.lock().await.insert(
                        new_sid.clone(),
                        McpSession {
                            caller: caller.clone(),
                            role: role.clone(),
                            created_at: Utc::now(),
                            tx,
                        },
                    );
                    session_id = Some(new_sid);
                } else {
                    return unauthorized_response(host, scheme);
                }
            }
        }
    }

    // Initialize: Auth erforderlich (401 triggert OAuth-Flow in claude.ai)
    if has_init {
        let init_rpc = requests.iter().find(|r| r.method == "initialize").unwrap();
        let id = init_rpc.id.clone().unwrap_or(serde_json::Value::Null);
        let new_session_id = Uuid::new_v4().to_string();
        let (tx, _) = broadcast::channel::<String>(100);
        let (caller, caller_role) = match &auth_result {
            Ok(pair) => pair.clone(),
            Err(_) => {
                // Kein Token → 401 mit WWW-Authenticate damit OAuth-Clients den Auth-Flow starten
                return unauthorized_response(host, scheme);
            }
        };
        let session = McpSession {
            caller,
            role: caller_role,
            created_at: Utc::now(),
            tx,
        };
        state
            .mcp_sessions
            .lock()
            .await
            .insert(new_session_id.clone(), session);

        // protocolVersion vom Client übernehmen (Kompatibilität mit 2024-11-05 und 2025-03-26)
        let client_version = init_rpc
            .params
            .get("protocolVersion")
            .and_then(|v| v.as_str())
            .unwrap_or("2024-11-05");
        let resp = JsonRpcResponse {
            jsonrpc: "2.0".into(),
            result: Some(serde_json::json!({
                "protocolVersion": client_version,
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": { "name": "plankton", "version": "1.0.0" }
            })),
            error: None,
            id,
        };

        return (
            [(
                header::HeaderName::from_static("mcp-session-id"),
                new_session_id,
            )],
            Json(resp),
        )
            .into_response();
    }

    // Auth: Token aus Header ODER Caller aus bestehender Session (Fallback)
    let (caller, caller_role) = if let Ok(pair) = auth_result {
        pair
    } else if let Some(ref sid) = session_id {
        // Fallback: Caller aus Session (für Clients die Token nur beim Init senden)
        let sessions = state.mcp_sessions.lock().await;
        if let Some(session) = sessions.get(sid) {
            if !session.caller.is_empty() && !session.role.is_empty() {
                (session.caller.clone(), session.role.clone())
            } else {
                return unauthorized_response(host, scheme);
            }
        } else {
            return unauthorized_response(host, scheme);
        }
    } else {
        return unauthorized_response(host, scheme);
    };

    // Alle Requests verarbeiten
    let mut responses: Vec<JsonRpcResponse> = Vec::new();
    for rpc in &requests {
        if let Some(resp) = handle_single_rpc(&state, rpc, &caller, &caller_role, &session_id).await
        {
            responses.push(resp);
        }
    }

    // Nur Notifications (keine Responses) → sofort 202 Accepted, egal ob SSE oder nicht
    if responses.is_empty() {
        let sid_header = session_id.unwrap_or_default();
        return (
            StatusCode::ACCEPTED,
            [(
                header::HeaderName::from_static("mcp-session-id"),
                sid_header,
            )],
        )
            .into_response();
    }

    // SSE-Modus: Antworten als SSE-Events streamen (nur initiale Events, kein long-lived Stream)
    // Long-lived SSE-Streams gehören zu GET /mcp, nicht zu POST Request-Response-Calls
    if use_sse {
        let initial_events: Vec<Result<Event, std::convert::Infallible>> = responses
            .into_iter()
            .filter_map(|r| {
                serde_json::to_string(&r)
                    .ok()
                    .map(|json| Ok(Event::default().event("message").data(json)))
            })
            .collect();

        let initial_stream = futures::stream::iter(initial_events);
        let mut resp = Sse::new(initial_stream).into_response();
        if let Some(sid) = session_id {
            resp.headers_mut().insert(
                header::HeaderName::from_static("mcp-session-id"),
                sid.parse().unwrap_or_else(|_| "invalid".parse().unwrap()),
            );
        }
        return resp;
    }

    // JSON-Modus: Einzelne Antwort oder Batch
    let sid_header = session_id.unwrap_or_default();
    if responses.len() == 1 {
        (
            [(
                header::HeaderName::from_static("mcp-session-id"),
                sid_header,
            )],
            Json(responses.into_iter().next().unwrap()),
        )
            .into_response()
    } else if responses.is_empty() {
        // Nur Notifications – kein Body, 202 Accepted
        (
            StatusCode::ACCEPTED,
            [(
                header::HeaderName::from_static("mcp-session-id"),
                sid_header,
            )],
        )
            .into_response()
    } else {
        (
            [(
                header::HeaderName::from_static("mcp-session-id"),
                sid_header,
            )],
            Json(responses),
        )
            .into_response()
    }
}

/// GET /mcp – SSE-Stream für Server-initiierte Nachrichten (Streamable HTTP Transport).
pub async fn mcp_sse_stream(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    // Auth prüfen: 401 mit WWW-Authenticate wenn kein gültiger Token
    if resolve_caller(&headers, &state).await.is_err() {
        let scheme = headers
            .get("x-forwarded-proto")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("http");
        let host = headers
            .get("host")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("localhost");
        return unauthorized_response(host, scheme);
    }

    let session_id = match headers.get("mcp-session-id").and_then(|v| v.to_str().ok()) {
        Some(sid) => sid.to_string(),
        None => return StatusCode::BAD_REQUEST.into_response(),
    };

    let sessions = state.mcp_sessions.lock().await;
    let rx = match sessions.get(&session_id) {
        Some(session) => session.tx.subscribe(),
        None => return StatusCode::NOT_FOUND.into_response(),
    };
    drop(sessions);

    let out = stream::unfold(rx, move |mut rx| async move {
        match rx.recv().await {
            Ok(msg) => Some((
                Ok::<_, std::convert::Infallible>(Event::default().data(msg)),
                rx,
            )),
            Err(broadcast::error::RecvError::Lagged(_)) => Some((
                Ok::<_, std::convert::Infallible>(Event::default().event("heartbeat").data("ping")),
                rx,
            )),
            Err(broadcast::error::RecvError::Closed) => None,
        }
    });
    Sse::new(out).into_response()
}

/// DELETE /mcp – MCP-Session beenden (Streamable HTTP Transport).
pub async fn mcp_session_delete(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> axum::http::StatusCode {
    use axum::http::StatusCode;

    let session_id = match headers.get("mcp-session-id").and_then(|v| v.to_str().ok()) {
        Some(sid) => sid.to_string(),
        None => return StatusCode::BAD_REQUEST,
    };

    let mut sessions = state.mcp_sessions.lock().await;
    if sessions.remove(&session_id).is_some() {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

/// Zentraler Tool-Executor für Legacy und JSON-RPC MCP. Öffentlich für Integrationstests.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) async fn execute_tool_pub(
    state: &AppState,
    tool: &str,
    args: &serde_json::Value,
    caller: &str,
) -> Result<serde_json::Value, ApiError> {
    execute_tool(state, tool, args, caller).await
}

async fn execute_tool(
    state: &AppState,
    tool: &str,
    args: &serde_json::Value,
    caller: &str,
) -> Result<serde_json::Value, ApiError> {
    match tool {
        "list_projects" => {
            // Nur Metadaten: id, title, slug, task_count, column_count
            let projects = state.store.list_projects().await?;
            let summary: Vec<serde_json::Value> = projects
                .iter()
                .map(|p| {
                    serde_json::json!({
                        "id": p.id,
                        "title": p.title,
                        "slug": p.slug,
                        "task_count": p.tasks.len(),
                        "column_count": p.columns.len(),
                    })
                })
                .collect();
            Ok(serde_json::to_value(summary)?)
        }
        "get_project" => {
            let id = args["id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("id missing".into()))?;
            let project = state.store.get_project(id).await?;
            // Kompakt: Spalten + Tasks ohne Logs/Comments
            let columns: Vec<serde_json::Value> = project
                .columns
                .iter()
                .filter(|c| !c.hidden)
                .map(|c| serde_json::json!({"id": c.id, "title": c.title, "order": c.order}))
                .collect();
            let tasks: Vec<serde_json::Value> = project
                .tasks
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "id": t.id, "title": t.title, "description": t.description,
                        "column_id": t.column_id, "labels": t.labels, "worker": t.worker,
                        "points": t.points, "task_type": t.task_type, "parent_id": t.parent_id,
                        "order": t.order,
                    })
                })
                .collect();
            Ok(serde_json::json!({
                "id": project.id, "title": project.title, "slug": project.slug,
                "columns": columns, "tasks": tasks,
            }))
        }
        "get_task" => {
            let project_id = args["project_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("project_id missing".into()))?;
            let task_id = args["task_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("task_id missing".into()))?;
            let project = state.store.get_project(project_id).await?;
            let task = project
                .tasks
                .iter()
                .find(|t| t.id == task_id)
                .ok_or_else(|| ApiError::NotFound("Task not found".into()))?;
            Ok(serde_json::to_value(task)?)
        }
        "create_project" => {
            let title = args["title"].as_str().unwrap_or("Untitled Project");
            let project = default_project(title.to_string());
            Ok(serde_json::to_value(
                state.store.create_project(project).await?,
            )?)
        }
        "update_project" => {
            let project_id = args["project_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("project_id missing".into()))?;
            let real_id = state.store.resolve_project_id(project_id).await?;
            let mut project = state.store.get_project(&real_id).await?;
            if let Some(new_title) = args["title"].as_str() {
                project.title = new_title.to_string();
                // Regenerate slug when title changes
                let base_slug = project_slugify(&project.title);
                let existing = state.store.list_projects().await?;
                let mut slug = base_slug.clone();
                let mut idx = 2;
                loop {
                    let conflict = existing.iter().any(|p| p.id != real_id && p.slug == slug);
                    if !conflict {
                        break;
                    }
                    slug = format!("{}-{}", base_slug, idx);
                    idx += 1;
                }
                project.slug = slug;
            }
            if let Some(new_owner) = args["owner"].as_str() {
                project.owner = if new_owner.is_empty() {
                    None
                } else {
                    Some(new_owner.to_string())
                };
            }
            if let Some(new_type) = args["type"].as_str() {
                // Nur bekannte Werte akzeptieren; unbekannte werden auf den Default "kanban" normalisiert.
                let normalized = match new_type {
                    "list" => Some("list".to_string()),
                    _ if new_type.is_empty() => None,
                    _ => Some(project.project_type().to_string()),
                };
                project.r#type = normalized;
            }
            if let Some(done_expire) = args["done_expire"].as_i64() {
                project.done_expire = Some(done_expire as i32);
            }
            if let Some(archive_delete) = args["archive_delete"].as_i64() {
                project.archive_delete = Some(archive_delete as i32);
            }
            if let Some(pinned) = args["pinned"].as_bool() {
                project.pinned = Some(pinned);
            }
            let updated = state.store.put_project(project).await?;
            publish_update(state, &real_id).await;
            Ok(serde_json::to_value(&updated)?)
        }
        "create_task" => {
            let project_id = args["project_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("project_id missing".into()))?;
            let mut project = state.store.get_project(project_id).await?;
            let now = Utc::now().to_rfc3339();
            let task = Task {
                id: Uuid::new_v4().to_string(),
                title: args["title"].as_str().unwrap_or("New task").to_string(),
                description: args["description"].as_str().unwrap_or("").to_string(),
                column_id: args["column_id"]
                    .as_str()
                    .unwrap_or(project.columns.first().map(|c| c.id.as_str()).unwrap_or(""))
                    .to_string(),
                creator: caller.to_string(),
                order: project.tasks.len() as i32,
                created_at: now.clone(),
                updated_at: now,
                labels: args["labels"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default(),
                worker: args["worker"].as_str().unwrap_or("").to_string(),
                points: args["points"].as_i64().unwrap_or(0) as i32,
                task_type: args["task_type"].as_str().unwrap_or("task").to_string(),
                parent_id: args["parent_id"].as_str().unwrap_or("").to_string(),
                ..Task::default()
            };
            project.tasks.push(task.clone());
            state.store.put_project(project).await?;
            publish_event(
                state,
                project_id,
                "task_created",
                serde_json::to_value(&task)?,
            )
            .await;
            Ok(serde_json::to_value(&task)?)
        }
        "update_task" => {
            let project_id = args["project_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("project_id missing".into()))?;
            let task_id = args["task_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("task_id missing".into()))?;
            // Layer 1: Per-Projekt Write-Lock
            let lock = state.get_project_write_lock(project_id).await;
            let _guard = lock.lock().await;
            let mut project = state.store.get_project(project_id).await?;
            // Layer 2: Optionaler _rev-Check
            if let Some(client_rev) = args["_rev"].as_str() {
                let current_rev = project.rev.clone().unwrap_or_else(|| "0".into());
                if client_rev != current_rev {
                    return Err(ApiError::Conflict(format!(
                        "conflict: current_rev is {current_rev}"
                    )));
                }
            }
            if let Some(task) = project.tasks.iter_mut().find(|t| t.id == task_id) {
                if let Some(title) = args["title"].as_str() {
                    task.title = title.to_string();
                }
                if let Some(desc) = args["description"].as_str() {
                    task.description = desc.to_string();
                }
                if let Some(labels) = args["labels"].as_array() {
                    task.labels = labels
                        .iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect();
                }
                if let Some(worker) = args["worker"].as_str() {
                    task.worker = worker.to_string();
                }
                if let Some(points) = args["points"].as_i64() {
                    task.points = points as i32;
                }
                if let Some(task_type) = args["task_type"].as_str() {
                    task.task_type = task_type.to_string();
                }
                if let Some(parent_id) = args["parent_id"].as_str() {
                    task.parent_id = parent_id.to_string();
                }
                task.updated_at = Utc::now().to_rfc3339();
            }
            let task_data = project.tasks.iter().find(|t| t.id == task_id).cloned();
            state.store.put_project(project).await?;
            if let Some(ref t) = task_data {
                publish_event(state, project_id, "task_updated", serde_json::to_value(t)?).await;
            }
            Ok(serde_json::to_value(&task_data)?)
        }
        "move_task" => {
            let project_id = args["project_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("project_id missing".into()))?;
            let task_id = args["task_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("task_id missing".into()))?;
            let column_id = args["column_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("column_id missing".into()))?;
            // Layer 1: Per-Projekt Write-Lock
            let lock = state.get_project_write_lock(project_id).await;
            let _guard = lock.lock().await;
            let mut project = state.store.get_project(project_id).await?;
            // Layer 2: Optionaler _rev-Check
            if let Some(client_rev) = args["_rev"].as_str() {
                let current_rev = project.rev.clone().unwrap_or_else(|| "0".into());
                if client_rev != current_rev {
                    return Err(ApiError::Conflict(format!(
                        "conflict: current_rev is {current_rev}"
                    )));
                }
            }
            let col_name = |cid: &str| -> String {
                project
                    .columns
                    .iter()
                    .find(|c| c.id == cid)
                    .map(|c| c.title.clone())
                    .unwrap_or_else(|| cid.to_string())
            };
            // Blocking-Enforcement: Move nach "In Progress" prüft blocked_by
            let target_col_title = col_name(column_id);
            if target_col_title == "In Progress" {
                if let Some(task) = project.tasks.iter().find(|t| t.id == task_id) {
                    if !task.blocked_by.is_empty() {
                        let done_col_id = project
                            .columns
                            .iter()
                            .find(|c| c.title == "Done")
                            .map(|c| c.id.clone());
                        let open_blockers: Vec<String> = task
                            .blocked_by
                            .iter()
                            .filter_map(|bid| {
                                project.tasks.iter().find(|t| &t.id == bid).and_then(|bt| {
                                    let is_done = done_col_id
                                        .as_deref()
                                        .map(|did| bt.column_id == did)
                                        .unwrap_or(false);
                                    if is_done {
                                        None
                                    } else {
                                        Some(bt.title.clone())
                                    }
                                })
                            })
                            .collect();
                        if !open_blockers.is_empty() {
                            return Err(ApiError::BadRequest(format!(
                                "Task is blocked by: {}",
                                open_blockers.join(", ")
                            )));
                        }
                    }
                }
            }
            if let Some(task) = project.tasks.iter_mut().find(|t| t.id == task_id) {
                let _old_name = col_name(&task.column_id);
                let new_name = col_name(column_id);
                task.previous_row = task.column_id.clone();
                task.column_id = column_id.to_string();
                if let Some(order) = args["order"].as_i64() {
                    task.order = order as i32;
                }
                let now = Utc::now();
                task.updated_at = now.to_rfc3339();
                task.column_entered_at = Some(now);
                task.logs
                    .push(log_entry(caller, &format!("→ {}", new_name)));
            }
            let task_data = project.tasks.iter().find(|t| t.id == task_id).cloned();
            state.store.put_project(project).await?;
            if let Some(ref t) = task_data {
                publish_event(state, project_id, "task_moved", serde_json::to_value(t)?).await;
            }
            Ok(serde_json::to_value(&task_data)?)
        }
        "delete_task" => {
            let project_id = args["project_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("project_id missing".into()))?;
            let task_id = args["task_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("task_id missing".into()))?;
            // Layer 1: Per-Projekt Write-Lock
            let lock = state.get_project_write_lock(project_id).await;
            let _guard = lock.lock().await;
            let mut project = state.store.get_project(project_id).await?;
            // Layer 2: Optionaler _rev-Check
            if let Some(client_rev) = args["_rev"].as_str() {
                let current_rev = project.rev.clone().unwrap_or_else(|| "0".into());
                if client_rev != current_rev {
                    return Err(ApiError::Conflict(format!(
                        "conflict: current_rev is {current_rev}"
                    )));
                }
            }
            // Relationen aufräumen
            for task in &mut project.tasks {
                task.blocks.retain(|id| id != task_id);
                task.blocked_by.retain(|id| id != task_id);
                task.subtask_ids.retain(|id| id != task_id);
                if task.parent_id == task_id {
                    task.parent_id.clear();
                }
            }
            project.tasks.retain(|t| t.id != task_id);
            state.store.put_project(project).await?;
            publish_event(
                state,
                project_id,
                "task_deleted",
                serde_json::json!({ "task_id": task_id }),
            )
            .await;
            Ok(serde_json::json!({"deleted": task_id}))
        }
        "summarize_board" => {
            let project_id = args["project_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("project_id missing".into()))?;
            let project = state.store.get_project(project_id).await?;
            let summary: Vec<_> = project
                .columns
                .iter()
                .filter(|c| !c.hidden)
                .map(|c| {
                    let count = project.tasks.iter().filter(|t| t.column_id == c.id).count();
                    serde_json::json!({"column": c.title, "tasks": count})
                })
                .collect();
            Ok(serde_json::json!({"project": project.title, "columns": summary}))
        }
        "list_epics" => {
            let project_id = args["project_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("project_id missing".into()))?;
            let project = state.store.get_project(project_id).await?;
            let mut visible_cols: Vec<_> = project.columns.iter().filter(|c| !c.hidden).collect();
            visible_cols.sort_by_key(|c| c.order);
            let epics: Vec<_> = visible_cols
                .iter()
                .map(|c| {
                    let count = project.tasks.iter().filter(|t| t.column_id == c.id).count();
                    serde_json::json!({"id": c.id, "title": c.title, "order": c.order, "task_count": count})
                })
                .collect();
            Ok(serde_json::json!({"project": project.title, "epics": epics}))
        }
        "assign_task" => {
            let project_id = args["project_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("project_id missing".into()))?;
            let task_id = args["task_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("task_id missing".into()))?;
            let worker = args["worker"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("worker missing".into()))?;
            // Layer 1: Per-Projekt Write-Lock
            let lock = state.get_project_write_lock(project_id).await;
            let _guard = lock.lock().await;
            let mut project = state.store.get_project(project_id).await?;
            // Layer 2: Optionaler _rev-Check
            if let Some(client_rev) = args["_rev"].as_str() {
                let current_rev = project.rev.clone().unwrap_or_else(|| "0".into());
                if client_rev != current_rev {
                    return Err(ApiError::Conflict(format!(
                        "conflict: current_rev is {current_rev}"
                    )));
                }
            }
            if let Some(task) = project.tasks.iter_mut().find(|t| t.id == task_id) {
                task.worker = worker.to_string();
                task.updated_at = Utc::now().to_rfc3339();
                task.logs
                    .push(log_entry(caller, &format!("assigned → {}", worker)));
            } else {
                return Err(ApiError::NotFound("Task not found".into()));
            }
            let task_data = project.tasks.iter().find(|t| t.id == task_id).cloned();
            let _updated = state.store.put_project(project).await?;
            if let Some(t) = task_data {
                publish_event(state, project_id, "task_updated", serde_json::to_value(&t)?).await;
            }
            Ok(serde_json::json!({"ok": true, "task_id": task_id}))
        }
        "get_assigned_tasks" => {
            let project_id = args["project_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("project_id missing".into()))?;
            let project = state.store.get_project(project_id).await?;
            let tasks: Vec<_> = project
                .tasks
                .iter()
                .filter(|t| t.worker == caller || t.creator == caller)
                .map(|t| serde_json::to_value(t).unwrap_or_default())
                .collect();
            Ok(serde_json::json!({"tasks": tasks}))
        }
        "add_log" => {
            // DEPRECATED: add_log is now an alias for add_comment (Ansatz A).
            // Agent calls to add_log are routed to task.comments for visibility.
            // Internal system events are written by Plankton itself (move, create, etc.).
            let project_id = args["project_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("project_id missing".into()))?;
            let task_id = args["task_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("task_id missing".into()))?;
            let message = args["message"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("message missing".into()))?;
            let mut project = state.store.get_project(project_id).await?;
            if let Some(task) = project.tasks.iter_mut().find(|t| t.id == task_id) {
                // Route to comments (same as add_comment) for agent-visible output
                task.comments.push(log_entry(caller, message));
                task.updated_at = Utc::now().to_rfc3339();
            } else {
                return Err(ApiError::NotFound("Task not found".into()));
            }
            let task_data = project.tasks.iter().find(|t| t.id == task_id).cloned();
            state.store.put_project(project).await?;
            if let Some(t) = task_data {
                publish_event(state, project_id, "task_updated", serde_json::to_value(&t)?).await;
            }
            Ok(
                serde_json::json!({"ok": true, "note": "add_log is deprecated, use add_comment instead"}),
            )
        }
        "submit_for_review" => {
            let project_id = args["project_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("project_id missing".into()))?;
            let task_id = args["task_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("task_id missing".into()))?;
            let mut project = state.store.get_project(project_id).await?;
            // Find the "In Progress" and "Testing" columns by title
            let in_progress_col = project
                .columns
                .iter()
                .find(|c| c.title == "In Progress")
                .map(|c| c.id.clone());
            let testing_col = project
                .columns
                .iter()
                .find(|c| c.title == "Testing")
                .map(|c| c.id.clone());
            if let Some(task) = project.tasks.iter_mut().find(|t| t.id == task_id) {
                // Validate that task is in "In Progress" column
                if let Some(ref in_progress_id) = in_progress_col {
                    if &task.column_id != in_progress_id {
                        let current_col_name = project
                            .columns
                            .iter()
                            .find(|c| c.id == task.column_id)
                            .map(|c| c.title.as_str())
                            .unwrap_or("Unknown");
                        return Err(ApiError::BadRequest(format!(
                            "Task must be in 'In Progress' before submitting for review. Current column: '{}'. Move the task to 'In Progress' first.",
                            current_col_name
                        )));
                    }
                }
                if !task.labels.contains(&"review".to_string()) {
                    task.labels.push("review".to_string());
                }
                // Move to Testing column if it exists
                if let Some(ref testing_id) = testing_col {
                    task.previous_row = task.column_id.clone();
                    task.column_id = testing_id.clone();
                    let now = Utc::now();
                    task.updated_at = now.to_rfc3339();
                    task.column_entered_at = Some(now);
                } else {
                    task.updated_at = Utc::now().to_rfc3339();
                }
                task.logs
                    .push(log_entry(caller, "submitted for review → Testing"));
            } else {
                return Err(ApiError::NotFound("Task not found".into()));
            }
            let task_data = project.tasks.iter().find(|t| t.id == task_id).cloned();
            state.store.put_project(project).await?;
            if let Some(t) = task_data {
                publish_event(state, project_id, "task_moved", serde_json::to_value(&t)?).await;
            }
            Ok(serde_json::json!({"ok": true, "task_id": task_id}))
        }
        "get_review_queue" => {
            let project_id = args["project_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("project_id missing".into()))?;
            let project = state.store.get_project(project_id).await?;
            let tasks: Vec<_> = project
                .tasks
                .iter()
                .filter(|t| t.labels.contains(&"review".to_string()))
                .map(|t| serde_json::to_value(t).unwrap_or_default())
                .collect();
            Ok(serde_json::json!({"tasks": tasks}))
        }
        "add_comment" => {
            let project_id = args["project_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("project_id missing".into()))?;
            let task_id = args["task_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("task_id missing".into()))?;
            let text = args["text"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("text missing".into()))?;
            let mut project = state.store.get_project(project_id).await?;
            if let Some(task) = project.tasks.iter_mut().find(|t| t.id == task_id) {
                task.comments.push(log_entry(caller, text));
                task.updated_at = Utc::now().to_rfc3339();
            } else {
                return Err(ApiError::NotFound("Task not found".into()));
            }
            let task_data = project.tasks.iter().find(|t| t.id == task_id).cloned();
            state.store.put_project(project).await?;
            if let Some(t) = task_data {
                publish_event(state, project_id, "task_updated", serde_json::to_value(&t)?).await;
            }
            Ok(serde_json::json!({"ok": true}))
        }
        "approve_task" => {
            let project_id = args["project_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("project_id missing".into()))?;
            let task_id = args["task_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("task_id missing".into()))?;
            let mut project = state.store.get_project(project_id).await?;
            let done_col = project
                .columns
                .iter()
                .find(|c| c.title == "Done")
                .map(|c| c.id.clone());
            if let Some(task) = project.tasks.iter_mut().find(|t| t.id == task_id) {
                task.labels.retain(|l| l != "review");
                if let Some(ref done_id) = done_col {
                    task.previous_row = task.column_id.clone();
                    task.column_id = done_id.clone();
                    let now = Utc::now();
                    task.updated_at = now.to_rfc3339();
                    task.column_entered_at = Some(now);
                } else {
                    task.updated_at = Utc::now().to_rfc3339();
                }
                task.logs.push(log_entry(caller, "✓ approved"));
            } else {
                return Err(ApiError::NotFound("Task not found".into()));
            }
            let task_data = project.tasks.iter().find(|t| t.id == task_id).cloned();
            state.store.put_project(project).await?;
            if let Some(t) = task_data {
                publish_event(state, project_id, "task_moved", serde_json::to_value(&t)?).await;
            }
            Ok(serde_json::json!({"ok": true, "task_id": task_id}))
        }
        "reject_task" => {
            let project_id = args["project_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("project_id missing".into()))?;
            let task_id = args["task_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("task_id missing".into()))?;
            let comment = args["comment"].as_str().unwrap_or("Rejected");
            let mut project = state.store.get_project(project_id).await?;
            if let Some(task) = project.tasks.iter_mut().find(|t| t.id == task_id) {
                task.labels.retain(|l| l != "review");
                if !task.previous_row.is_empty() {
                    let prev = task.previous_row.clone();
                    task.column_id = prev;
                    let now = Utc::now();
                    task.updated_at = now.to_rfc3339();
                    task.column_entered_at = Some(now);
                } else {
                    task.updated_at = Utc::now().to_rfc3339();
                }
                task.comments.push(log_entry(caller, comment));
                task.logs
                    .push(log_entry(caller, &format!("✗ rejected: {}", comment)));
            } else {
                return Err(ApiError::NotFound("Task not found".into()));
            }
            let task_data = project.tasks.iter().find(|t| t.id == task_id).cloned();
            state.store.put_project(project).await?;
            if let Some(t) = task_data {
                publish_event(state, project_id, "task_moved", serde_json::to_value(&t)?).await;
            }
            Ok(serde_json::json!({"ok": true, "task_id": task_id}))
        }
        "list_subtasks" => {
            let project_id = args["project_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("project_id missing".into()))?;
            let parent_id = args["parent_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("parent_id missing".into()))?;
            let project = state.store.get_project(project_id).await?;
            let done_col = project
                .columns
                .iter()
                .find(|c| c.title == "Done")
                .map(|c| c.id.as_str());
            let subtasks: Vec<_> = project
                .tasks
                .iter()
                .filter(|t| t.parent_id == parent_id)
                .map(|t| {
                    let is_done = done_col.map(|d| t.column_id == d).unwrap_or(false);
                    let col_name = project
                        .columns
                        .iter()
                        .find(|c| c.id == t.column_id)
                        .map(|c| c.title.as_str())
                        .unwrap_or("?");
                    serde_json::json!({
                        "id": t.id, "title": t.title, "task_type": t.task_type,
                        "column": col_name, "worker": t.worker, "done": is_done
                    })
                })
                .collect();
            Ok(serde_json::json!({"parent_id": parent_id, "subtasks": subtasks}))
        }
        "add_relation" => {
            let project_id = args["project_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("project_id missing".into()))?;
            let from_id = args["from_task_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("from_task_id missing".into()))?;
            let to_id = args["to_task_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("to_task_id missing".into()))?;
            let relation = args["relation"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("relation missing (blocks|subtask)".into()))?;
            let mut project = state.store.get_project(project_id).await?;
            match relation {
                "blocks" => {
                    if let Some(t) = project.tasks.iter_mut().find(|t| t.id == from_id) {
                        if !t.blocks.contains(&to_id.to_string()) {
                            t.blocks.push(to_id.to_string());
                        }
                    }
                    if let Some(t) = project.tasks.iter_mut().find(|t| t.id == to_id) {
                        if !t.blocked_by.contains(&from_id.to_string()) {
                            t.blocked_by.push(from_id.to_string());
                        }
                    }
                }
                "subtask" => {
                    // from = parent (epic), to = child (subtask)
                    if let Some(t) = project.tasks.iter_mut().find(|t| t.id == from_id) {
                        if !t.subtask_ids.contains(&to_id.to_string()) {
                            t.subtask_ids.push(to_id.to_string());
                        }
                    }
                    if let Some(t) = project.tasks.iter_mut().find(|t| t.id == to_id) {
                        t.parent_id = from_id.to_string();
                    }
                }
                _ => {
                    return Err(ApiError::BadRequest(format!(
                        "unknown relation: {relation} (use blocks|subtask)"
                    )))
                }
            }
            state.store.put_project(project).await?;
            publish_update(state, project_id).await;
            Ok(serde_json::json!({"ok": true, "relation": relation, "from": from_id, "to": to_id}))
        }
        "remove_relation" => {
            let project_id = args["project_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("project_id missing".into()))?;
            let from_id = args["from_task_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("from_task_id missing".into()))?;
            let to_id = args["to_task_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("to_task_id missing".into()))?;
            let relation = args["relation"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("relation missing (blocks|subtask)".into()))?;
            let mut project = state.store.get_project(project_id).await?;
            match relation {
                "blocks" => {
                    if let Some(t) = project.tasks.iter_mut().find(|t| t.id == from_id) {
                        t.blocks.retain(|id| id != to_id);
                    }
                    if let Some(t) = project.tasks.iter_mut().find(|t| t.id == to_id) {
                        t.blocked_by.retain(|id| id != from_id);
                    }
                }
                "subtask" => {
                    if let Some(t) = project.tasks.iter_mut().find(|t| t.id == from_id) {
                        t.subtask_ids.retain(|id| id != to_id);
                    }
                    if let Some(t) = project.tasks.iter_mut().find(|t| t.id == to_id) {
                        t.parent_id.clear();
                    }
                }
                _ => {
                    return Err(ApiError::BadRequest(format!(
                        "unknown relation: {relation}"
                    )))
                }
            }
            state.store.put_project(project).await?;
            publish_update(state, project_id).await;
            Ok(serde_json::json!({"ok": true}))
        }
        "reorder_tasks" => {
            let project_id = args["project_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("project_id missing".into()))?;
            let column_id = args["column_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("column_id missing".into()))?;
            let task_ids = args["task_ids"].as_array().ok_or_else(|| {
                ApiError::BadRequest("task_ids missing (array of task IDs in desired order)".into())
            })?;
            let mut project = state.store.get_project(project_id).await?;
            let mut reordered = 0;
            for (i, tid_val) in task_ids.iter().enumerate() {
                if let Some(tid) = tid_val.as_str() {
                    if let Some(task) = project
                        .tasks
                        .iter_mut()
                        .find(|t| t.id == tid && t.column_id == column_id)
                    {
                        task.order = i as i32;
                        task.updated_at = Utc::now().to_rfc3339();
                        reordered += 1;
                    }
                }
            }
            state.store.put_project(project).await?;
            publish_update(state, project_id).await;
            Ok(serde_json::json!({"ok": true, "reordered": reordered}))
        }
        "create_task_from_template" => {
            let project_id = args["project_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("project_id missing".into()))?;
            let template_name = args["template_name"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("template_name missing".into()))?;
            let title = args["title"].as_str().unwrap_or("").to_string();

            // Template laden (lokale Datei hat Vorrang vor eingebetteten Defaults)
            let tmpl = load_template(template_name)?;

            // Variable-Substitution im Beschreibungstext und Titel
            let today = Utc::now().format("%Y-%m-%d").to_string();
            let raw_title = tmpl
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            // Wenn ein expliziter Titel übergeben wurde, wird er direkt verwendet.
            // Andernfalls wird der Template-Titel mit Variable-Substitution genutzt.
            let resolved_title = if title.is_empty() {
                apply_template_vars(&raw_title, &title, &today)
            } else {
                title.clone()
            };
            let raw_desc = tmpl
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let resolved_desc = apply_template_vars(&raw_desc, &title, &today);

            // Labels aus Template + optionale extras
            let tmpl_labels: Vec<String> = tmpl
                .get("labels")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            let extra_labels: Vec<String> = args["labels"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            let mut labels = tmpl_labels;
            for l in extra_labels {
                if !labels.contains(&l) {
                    labels.push(l);
                }
            }

            let task_type = args["task_type"]
                .as_str()
                .or_else(|| tmpl.get("task_type").and_then(|v| v.as_str()))
                .unwrap_or("task")
                .to_string();

            let mut project = state.store.get_project(project_id).await?;
            let now = Utc::now().to_rfc3339();
            let task = Task {
                id: Uuid::new_v4().to_string(),
                title: resolved_title,
                description: resolved_desc,
                column_id: args["column_id"]
                    .as_str()
                    .unwrap_or(project.columns.first().map(|c| c.id.as_str()).unwrap_or(""))
                    .to_string(),
                creator: caller.to_string(),
                order: project.tasks.len() as i32,
                created_at: now.clone(),
                updated_at: now,
                labels,
                worker: args["worker"].as_str().unwrap_or("").to_string(),
                points: args["points"].as_i64().unwrap_or(0) as i32,
                task_type,
                parent_id: args["parent_id"].as_str().unwrap_or("").to_string(),
                ..Task::default()
            };
            project.tasks.push(task.clone());
            state.store.put_project(project).await?;
            publish_event(
                state,
                project_id,
                "task_created",
                serde_json::to_value(&task)?,
            )
            .await;
            Ok(serde_json::to_value(&task)?)
        }
        "move_task_to_project" => {
            let task_id = args["task_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("task_id missing".into()))?;
            let source_project_id = args["source_project_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("source_project_id missing".into()))?;
            let target_project_id = args["target_project_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("target_project_id missing".into()))?;

            // Guard: Kein Verschieben ins selbe Projekt
            if source_project_id == target_project_id {
                return Err(ApiError::BadRequest(
                    "Cannot move task to the same project".into(),
                ));
            }

            // Write-Locks für beide Projekte (immer in sortierter Reihenfolge um Deadlocks zu vermeiden)
            let (first_id, second_id) = if source_project_id < target_project_id {
                (source_project_id, target_project_id)
            } else {
                (target_project_id, source_project_id)
            };
            let lock1 = state.get_project_write_lock(first_id).await;
            let _guard1 = lock1.lock().await;
            let lock2 = state.get_project_write_lock(second_id).await;
            let _guard2 = lock2.lock().await;

            // Projekte laden
            let mut src_project = state.store.get_project(source_project_id).await?;
            let mut dst_project = state.store.get_project(target_project_id).await?;

            // Ziel-Projekt muss mindestens eine Spalte haben
            if dst_project.columns.is_empty() {
                return Err(ApiError::BadRequest("Target project has no columns".into()));
            }

            // Task aus Quellprojekt holen
            let task_pos = src_project
                .tasks
                .iter()
                .position(|t| t.id == task_id)
                .ok_or_else(|| ApiError::NotFound("Task not found in source project".into()))?;
            let task = src_project.tasks.remove(task_pos);

            // Spaltenname des Tasks im Quellprojekt bestimmen
            let src_col_title = src_project
                .columns
                .iter()
                .find(|c| c.id == task.column_id)
                .map(|c| c.title.clone())
                .unwrap_or_default();

            // Spalten-Mapping: gleicher Name im Zielprojekt suchen,
            // Fallback: erste Spalte (order=0)
            let target_col_id = dst_project
                .columns
                .iter()
                .find(|c| c.title == src_col_title)
                .or_else(|| dst_project.columns.iter().min_by_key(|c| c.order))
                .map(|c| c.id.clone())
                .expect("target project has at least one column (checked above)");

            // Task mit neuer project_id und column_id anlegen
            let new_task_id = Uuid::new_v4().to_string();
            let mut moved_task = task.clone();
            moved_task.id = new_task_id.clone();
            moved_task.column_id = target_col_id.clone();
            moved_task.previous_row = String::new();
            let now = Utc::now();
            moved_task.updated_at = now.to_rfc3339();
            moved_task.column_entered_at = Some(now);
            moved_task.logs.push(log_entry(
                caller,
                &format!(
                    "moved from project '{}' → '{}'",
                    src_project.title, dst_project.title
                ),
            ));

            // Task im Zielprojekt hinzufügen
            dst_project.tasks.push(moved_task);

            // Beide Projekte speichern
            state.store.put_project(src_project.clone()).await?;
            state.store.put_project(dst_project.clone()).await?;

            // SSE-Events publizieren
            publish_event(
                state,
                source_project_id,
                "task_deleted",
                serde_json::json!({ "task_id": task_id }),
            )
            .await;
            publish_event(
                state,
                target_project_id,
                "task_created",
                serde_json::json!({ "task_id": new_task_id }),
            )
            .await;

            Ok(serde_json::json!({
                "task_id": new_task_id,
                "column_id": target_col_id,
                "source_project_id": source_project_id,
                "target_project_id": target_project_id,
            }))
        }
        // ── File-Attachment Tools ────────────────────────────────────
        "attach_file" => {
            use base64::Engine;

            let attachment_store = state.attachment_store.as_ref().ok_or_else(|| {
                ApiError::BadRequest(
                    "File uploads not configured on this server (S3_BUCKET not set)".into(),
                )
            })?;

            let project_id = args["project_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("project_id missing".into()))?;
            let task_id = args["task_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("task_id missing".into()))?;
            let filename = args["filename"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("filename missing".into()))?;
            let content_b64 = args["content_base64"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("content_base64 missing".into()))?;

            let data = base64::engine::general_purpose::STANDARD
                .decode(content_b64)
                .map_err(|e| ApiError::BadRequest(format!("base64 decode error: {e}")))?;

            // 500 KB Limit für MCP-Upload
            const MAX_BYTES: usize = 500 * 1024;
            if data.len() > MAX_BYTES {
                return Err(ApiError::BadRequest(format!(
                    "File too large for MCP upload: {} bytes (max 500 KB = {} bytes). Use `plankton attach` CLI for larger files.",
                    data.len(),
                    MAX_BYTES
                )));
            }

            let mime_type = args["mime_type"]
                .as_str()
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    mime_guess::from_path(filename)
                        .first_or_octet_stream()
                        .to_string()
                });

            let attachment_id = Uuid::new_v4().to_string();
            let key = format!("{}/{}/{}/{}", project_id, task_id, attachment_id, filename);
            let size_bytes = data.len() as i64;

            let url = attachment_store.upload(&key, data, &mime_type).await?;

            let att = crate::models::AttachmentRef {
                id: attachment_id,
                filename: filename.to_string(),
                url,
                mime_type,
                size_bytes,
                created_at: Utc::now().to_rfc3339(),
            };

            // In Task persistieren
            let lock = state.get_project_write_lock(project_id).await;
            let _guard = lock.lock().await;
            let mut project = state.store.resolve_project(project_id).await?;
            let task = project
                .tasks
                .iter_mut()
                .find(|t| t.id == task_id || t.slug == task_id)
                .ok_or_else(|| ApiError::NotFound(format!("task {task_id} not found")))?;
            task.attachments.push(att.clone());
            state.store.put_project(project).await?;

            Ok(serde_json::to_value(&att)?)
        }

        "list_attachments" => {
            let project_id = args["project_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("project_id missing".into()))?;
            let task_id = args["task_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("task_id missing".into()))?;

            let project = state.store.resolve_project(project_id).await?;
            let task = project
                .tasks
                .iter()
                .find(|t| t.id == task_id || t.slug == task_id)
                .ok_or_else(|| ApiError::NotFound(format!("task {task_id} not found")))?;

            Ok(serde_json::to_value(&task.attachments)?)
        }

        "get_attachment" => {
            let attachment_store = state.attachment_store.as_ref().ok_or_else(|| {
                ApiError::BadRequest(
                    "File uploads not configured on this server (S3_BUCKET not set)".into(),
                )
            })?;

            let project_id = args["project_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("project_id missing".into()))?;
            let task_id = args["task_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("task_id missing".into()))?;
            let attachment_id = args["attachment_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("attachment_id missing".into()))?;

            let project = state.store.resolve_project(project_id).await?;
            let task = project
                .tasks
                .iter()
                .find(|t| t.id == task_id || t.slug == task_id)
                .ok_or_else(|| ApiError::NotFound(format!("task {task_id} not found")))?;
            let att = task
                .attachments
                .iter()
                .find(|a| a.id == attachment_id)
                .ok_or_else(|| {
                    ApiError::NotFound(format!("attachment {attachment_id} not found"))
                })?;

            let key = format!(
                "{}/{}/{}/{}",
                project_id, task_id, attachment_id, att.filename
            );
            let url = attachment_store.download_url(&key, 3600).await?;

            Ok(serde_json::json!({
                "id": att.id,
                "filename": att.filename,
                "mime_type": att.mime_type,
                "size_bytes": att.size_bytes,
                "url": url,
                "created_at": att.created_at,
            }))
        }

        "delete_attachment" => {
            let attachment_store = state.attachment_store.as_ref().ok_or_else(|| {
                ApiError::BadRequest(
                    "File uploads not configured on this server (S3_BUCKET not set)".into(),
                )
            })?;

            let project_id = args["project_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("project_id missing".into()))?;
            let task_id = args["task_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("task_id missing".into()))?;
            let attachment_id = args["attachment_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("attachment_id missing".into()))?;

            let lock = state.get_project_write_lock(project_id).await;
            let _guard = lock.lock().await;
            let mut project = state.store.resolve_project(project_id).await?;
            let task = project
                .tasks
                .iter_mut()
                .find(|t| t.id == task_id || t.slug == task_id)
                .ok_or_else(|| ApiError::NotFound(format!("task {task_id} not found")))?;

            let idx = task
                .attachments
                .iter()
                .position(|a| a.id == attachment_id)
                .ok_or_else(|| {
                    ApiError::NotFound(format!("attachment {attachment_id} not found"))
                })?;

            let att = task.attachments.remove(idx);
            state.store.put_project(project).await?;

            let key = format!(
                "{}/{}/{}/{}",
                project_id, task_id, attachment_id, att.filename
            );
            attachment_store.delete(&key).await?;

            Ok(serde_json::json!({ "ok": true }))
        }

        _ => Err(ApiError::BadRequest(format!("unknown tool: {tool}"))),
    }
}

/// Ersetzt `{{title}}` und `{{date}}` in einem Template-String.
fn apply_template_vars(text: &str, title: &str, date: &str) -> String {
    text.replace("{{title}}", title).replace("{{date}}", date)
}

/// Template-Daten: JSON-Objekt mit title, description, labels, task_type.
///
/// Lookup-Reihenfolge:
/// 1. `.plankton/templates/<name>.json` (lokale Datei, falls vorhanden)
/// 2. Eingebettete Standard-Templates
fn load_template(name: &str) -> Result<serde_json::Value, ApiError> {
    // 1. Lokale Datei prüfen
    let local_path = std::path::Path::new(".plankton/templates").join(format!("{name}.json"));
    if local_path.exists() {
        let content = std::fs::read_to_string(&local_path)
            .map_err(|e| ApiError::BadRequest(format!("failed to read template file: {e}")))?;
        let tmpl: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| ApiError::BadRequest(format!("invalid JSON in template file: {e}")))?;
        return Ok(tmpl);
    }

    // 2. Eingebettete Standard-Templates
    let tmpl = match name {
        "bug" => serde_json::json!({
            "title": "BUG: {{title}}",
            "task_type": "task",
            "labels": ["bug"],
            "description": "## Problem\n\n## Schritte zur Reproduktion\n\n1. \n2. \n\n## Erwartetes Verhalten\n\n## Tatsächliches Verhalten\n\n## Akzeptanzkriterien\n\n- [ ] Bug behoben\n- [ ] Kein Regressionsfehler"
        }),
        "feature" => serde_json::json!({
            "title": "FEATURE: {{title}}",
            "task_type": "task",
            "labels": ["feature"],
            "description": "## Anforderung\n\n## Hintergrund\n\n## Akzeptanzkriterien\n\n- [ ] Feature implementiert\n- [ ] Tests vorhanden\n- [ ] Dokumentation aktualisiert\n\n## Erstellt am\n\n{{date}}"
        }),
        "security" => serde_json::json!({
            "title": "SECURITY: {{title}}",
            "task_type": "task",
            "labels": ["security"],
            "description": "## Schwachstelle\n\n## Severity\n\n- [ ] Critical\n- [ ] High\n- [ ] Medium\n- [ ] Low\n\n## Betroffene Komponenten\n\n## Reproduktion\n\n## Gegenmaßnahmen\n\n## Akzeptanzkriterien\n\n- [ ] Schwachstelle behoben\n- [ ] Kein Regressionsfehler\n- [ ] Security-Review durchgeführt\n\n## Erstellt am\n\n{{date}}"
        }),
        "epic" => serde_json::json!({
            "title": "EPIC: {{title}}",
            "task_type": "epic",
            "labels": ["epic"],
            "description": "## Ziel\n\n## Hintergrund\n\n## Sub-Tasks\n\n- [ ] \n- [ ] \n\n## Akzeptanzkriterien\n\n- [ ] Alle Sub-Tasks abgeschlossen\n\n## Erstellt am\n\n{{date}}"
        }),
        "chore" => serde_json::json!({
            "title": "CHORE: {{title}}",
            "task_type": "task",
            "labels": ["chore"],
            "description": "## Aufgabe\n\n## Motivation\n\n## Definition of Done\n\n- [ ] Aufgabe erledigt\n\n## Erstellt am\n\n{{date}}"
        }),
        _ => {
            return Err(ApiError::BadRequest(format!(
                "template '{name}' not found (no local file at .plankton/templates/{name}.json and no built-in default)"
            )));
        }
    };
    Ok(tmpl)
}

/// GET /docs – Maschinenlesbare API-Dokumentation.
pub async fn docs_page() -> axum::response::Html<String> {
    axum::response::Html(generate_docs_html())
}

/// GET /skill.md – Claude Code Skill-Datei zum Download.
pub async fn skill_md(
    axum::extract::Host(host): axum::extract::Host,
    headers: axum::http::HeaderMap,
) -> impl axum::response::IntoResponse {
    // Plankton-URL aus dem Request ableiten.
    let scheme = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("http");
    let plankton_url = format!("{scheme}://{host}");

    let content = format!(
        r#"---
name: plankton
description: Manage tasks on the Plankton Kanban board – create, assign, review, and move tasks using curl and the JSON-RPC API
allowed-tools: Bash, Read, Grep, WebFetch
user-invocable: true
---

# Plankton – Kanban Board für KI-Agenten

Plankton ist ein Kanban-Board mit REST-API und JSON-RPC-Schnittstelle.
Du interagierst damit ausschließlich über **curl-Aufrufe** an den JSON-RPC-Endpunkt.

- **Dokumentation:** {plankton_url}/docs

## Multi-Server Secrets

Plankton unterstützt mehrere Server gleichzeitig. Die Secrets-Datei enthält pro Server einen Abschnitt.

Lies die Secrets-Datei aus einem der folgenden Orte (erster Treffer gewinnt):

1. `~/.claude/plankton_secrets.md` (persönlich, empfohlen)
2. `.claude/plankton_secrets.md` (projektlokal)
3. `~/.claude/plankton-secrets.md` (Legacy-Format, Fallback)

### Secrets-Format

```ini
# Plankton Server Tokens

[plankton.tiny-dev.de]
URL=https://plankton.tiny-dev.de
PLANKTON_TOKEN=eyJ...

[plankton.local:3000]
URL=http://plankton.local:3000
PLANKTON_TOKEN=eyJ...
```

### Server-Erkennung aus Ticket-URLs

Wenn der User eine Ticket-URL angibt (z.B. `https://plankton.tiny-dev.de/p/project/t/task-slug`),
extrahiere den Hostnamen (`plankton.tiny-dev.de`) und finde den passenden Abschnitt in der Secrets-Datei.
Verwende die `URL` aus diesem Abschnitt als Server-Basis und den `PLANKTON_TOKEN` als Bearer-Token.

Falls kein passender Server gefunden wird, informiere den User:
> Kein Token für Server `<hostname>` gefunden. Bitte `plankton skill install <url>` ausführen.

### Installation

```bash
# CLI installieren
curl -fsSL {plankton_url}/install | bash

# Skill installieren (inkl. Login + Secrets-Setup)
plankton skill install {plankton_url} --global
```

## API-Aufrufe

Plankton unterstützt **MCP Streamable HTTP Transport** (Protocol Version `2025-03-26`).
Alle Tool-Aufrufe gehen an `POST $PLANKTON_URL/mcp` als JSON-RPC 2.0.
Verwende `$PLANKTON_URL` (die URL aus dem passenden Secrets-Abschnitt) und `$PLANKTON_TOKEN` (den Token dazu).

### Streamable HTTP Transport (empfohlen)

```bash
# 1. Session initialisieren → Mcp-Session-Id aus Response-Header lesen
curl -s -D- -X POST $PLANKTON_URL/mcp \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $PLANKTON_TOKEN" \
  -d '{{"jsonrpc":"2.0","method":"initialize","id":0}}'
# → Header: Mcp-Session-Id: <session-id>

# 2. Tool aufrufen (mit Session-ID)
curl -s -X POST $PLANKTON_URL/mcp \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $PLANKTON_TOKEN" \
  -H "Mcp-Session-Id: <session-id>" \
  -d '{{"jsonrpc":"2.0","method":"tools/call","params":{{"name":"TOOL_NAME","arguments":{{ARGS}}}},"id":1}}'

# 3. SSE-Stream für Server-Notifications (optional)
curl -s -N $PLANKTON_URL/mcp \
  -H "Authorization: Bearer $PLANKTON_TOKEN" \
  -H "Mcp-Session-Id: <session-id>"

# 4. Session beenden
curl -s -X DELETE $PLANKTON_URL/mcp \
  -H "Mcp-Session-Id: <session-id>"
```

### Legacy-Aufruf (ohne Session)

```bash
curl -s -X POST $PLANKTON_URL/mcp \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $PLANKTON_TOKEN" \
  -d '{{"jsonrpc":"2.0","method":"tools/call","params":{{"name":"TOOL_NAME","arguments":{{ARGS}}}},"id":1}}'
```

Ersetze `TOOL_NAME` und `ARGS` mit den Werten aus der Tool-Referenz unten.
Die Antwort kommt als `{{"result":{{"content":[{{"type":"text","text":"..."}}]}}}}`.

## Tool-Referenz

Jedes Tool wird per `tools/call` aufgerufen. Hier sind alle Tools mit ihren Parametern:

### Öffentliche Tools (kein spezieller Token nötig)

**list_projects** – Alle Projekte auflisten
- Parameter: keine
- Beispiel: `{{"name":"list_projects","arguments":{{}}}}`

**get_project** – Ein Projekt mit allen Tasks laden
- Parameter: `id` (string, required) – Projekt-ID
- REST-API unterstützt zusätzliche Query-Parameter: `sort` ("order"|"title"|"created"|"updated"|"points"), `group_epics` (true/false)
- Beispiel: `{{"name":"get_project","arguments":{{"id":"PROJEKT_ID"}}}}`

**summarize_board** – Board-Übersicht mit Spalten und Task-Anzahl
- Parameter: `project_id` (string, required)
- Beispiel: `{{"name":"summarize_board","arguments":{{"project_id":"PROJEKT_ID"}}}}`

### Manager / Architect Tools

**create_project** – Neues Projekt anlegen
- Parameter: `title` (string, optional, default: "Untitled Project")

**list_epics** – Spalten als Epics mit Task-Anzahl anzeigen
- Parameter: `project_id` (string, required)

**create_task** – Neuen Task erstellen
- Parameter:
  - `project_id` (string, required)
  - `title` (string, optional)
  - `description` (string, optional)
  - `column_id` (string, optional – default: erste Spalte)
  - `labels` (string[], optional)
  - `worker` (string, optional)
  - `points` (number, optional)
  - `task_type` (string, optional – "task"|"epic"|"job", default: "task")
  - `parent_id` (string, optional – Parent-Epic-ID für Subtasks)
- Beispiel: `{{"name":"create_task","arguments":{{"project_id":"ID","title":"Feature X","task_type":"epic","labels":["feature"],"points":5}}}}`

**move_task** – Task in andere Spalte verschieben
- Parameter: `project_id`, `task_id`, `column_id` (alle string, required), `order` (number, optional)

**reorder_tasks** – Tasks innerhalb einer Spalte umsortieren
- Parameter: `project_id`, `column_id` (string, required), `task_ids` (string[], required – IDs in gewünschter Reihenfolge)

**assign_task** – Worker einem Task zuweisen
- Parameter: `project_id`, `task_id`, `worker` (alle string, required)

**delete_task** – Task löschen
- Parameter: `project_id`, `task_id` (beide string, required)

### Developer Tools

**get_assigned_tasks** – Dem Aufrufer zugewiesene Tasks
- Parameter: `project_id` (string, required)

**update_task** – Task bearbeiten
- Parameter:
  - `project_id` (string, required)
  - `task_id` (string, required)
  - `title` (string, optional)
  - `description` (string, optional)
  - `labels` (string[], optional)
  - `worker` (string, optional)
  - `points` (number, optional)
  - `task_type` (string, optional – "task"|"epic"|"job")
  - `parent_id` (string, optional – Parent-Epic-ID)

**add_log** – Log-Eintrag zu einem Task hinzufügen (nur für Status-Änderungen und Fortschritt, NICHT für Tester-Feedback)
- Parameter: `project_id`, `task_id`, `message` (alle string, required)

**submit_for_review** – Task zur Review einreichen (verschiebt nach Testing-Spalte)
- Parameter: `project_id`, `task_id` (beide string, required)

### Tester Tools

**Wichtig:** Tester schreiben Feedback und Testergebnisse immer als **Kommentar** (`add_comment`), niemals als Log (`add_log`). Logs sind für automatische Status-Änderungen reserviert.

**get_review_queue** – Tasks mit Label "review" auflisten
- Parameter: `project_id` (string, required)

**add_comment** – Kommentar zu einem Task hinzufügen (für Tester-Feedback, Fehlerberichte, Testergebnisse)
- Parameter: `project_id`, `task_id`, `text` (alle string, required)

**approve_task** – Task abnehmen (verschiebt nach "Done", entfernt "review"-Label)
- Parameter: `project_id`, `task_id` (beide string, required)

**reject_task** – Task zurückweisen (verschiebt zurück nach "In Progress", entfernt "review"-Label)
- Parameter: `project_id`, `task_id` (beide string, required), `comment` (string, optional)

### Relation Tools

**list_subtasks** – Subtasks eines Epics mit Fertigstellungsstatus auflisten
- Parameter: `project_id`, `parent_id` (beide string, required)
- Gibt zurück: Array mit `id`, `title`, `task_type`, `column`, `worker`, `done` (boolean)

**add_relation** – Relation zwischen zwei Tasks erstellen
- Parameter:
  - `project_id` (string, required)
  - `from_task_id` (string, required) – Bei "blocks": der blockierende Task; bei "subtask": der Parent-Epic
  - `to_task_id` (string, required) – Bei "blocks": der blockierte Task; bei "subtask": der Subtask
  - `relation` (string, required – "blocks"|"subtask")

**remove_relation** – Relation zwischen zwei Tasks entfernen
- Parameter: `project_id`, `from_task_id`, `to_task_id`, `relation` (alle string, required)

## Task-Typen

- **task** (Standard) – Normale Aufgabe
- **epic** – Große User-Story mit Subtasks. Hat `subtask_ids` und zeigt Fortschritt an.
- **job** – Automatisierte/wiederkehrende Aufgabe

## Task-Relationen

- **blocks** – Task A blockiert Task B (B kann nicht bearbeitet werden solange A nicht in Done ist)
- **subtask** – Task B ist Subtask von Epic A. Wird automatisch bidirektional gesetzt (parent_id ↔ subtask_ids)

## Typischer Workflow

1. `list_projects` → Projekt-ID finden
2. `get_project` → Spalten-IDs und Tasks sehen
3. `create_task` → Neuen Task anlegen
4. `move_task` → Task in "In Progress" verschieben
5. `add_log` → Fortschritt dokumentieren
6. `submit_for_review` → Zur Review einreichen
7. `approve_task` / `reject_task` → Review abschließen

## Vollständiges Beispiel (Streamable HTTP)

```bash
# PLANKTON_URL und PLANKTON_TOKEN aus Secrets laden (passend zum Ticket-Server)

# 1. Session initialisieren
SESSION=$(curl -s -D- -X POST $PLANKTON_URL/mcp \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $PLANKTON_TOKEN" \
  -d '{{"jsonrpc":"2.0","method":"initialize","id":0}}' \
  | grep -i mcp-session-id | tr -d '\\r' | awk '{{print $2}}')

# 2. Projekte auflisten
curl -s -X POST $PLANKTON_URL/mcp \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $PLANKTON_TOKEN" \
  -H "Mcp-Session-Id: $SESSION" \
  -d '{{"jsonrpc":"2.0","method":"tools/call","params":{{"name":"list_projects","arguments":{{}}}},"id":1}}'

# 3. Epic erstellen mit Subtask
curl -s -X POST $PLANKTON_URL/mcp \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $PLANKTON_TOKEN" \
  -H "Mcp-Session-Id: $SESSION" \
  -d '{{"jsonrpc":"2.0","method":"tools/call","params":{{"name":"create_task","arguments":{{"project_id":"PROJ_ID","title":"Auth System","task_type":"epic","labels":["feature"],"points":13}}}},"id":2}}'

# 4. Subtask-Relation anlegen
curl -s -X POST $PLANKTON_URL/mcp \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $PLANKTON_TOKEN" \
  -H "Mcp-Session-Id: $SESSION" \
  -d '{{"jsonrpc":"2.0","method":"tools/call","params":{{"name":"add_relation","arguments":{{"project_id":"PROJ_ID","from_task_id":"EPIC_ID","to_task_id":"SUBTASK_ID","relation":"subtask"}}}},"id":3}}'

# 5. Session beenden
curl -s -X DELETE $PLANKTON_URL/mcp -H "Mcp-Session-Id: $SESSION"
```

## Regeln

1. Jeder Agent arbeitet nur mit seinem Token und den damit verfügbaren Tools
2. Kommunikation erfolgt über Task-Kommentare und -Logs in Plankton
3. Der Workflow läuft vollständig autonom ohne Rückfragen an den Nutzer
4. Bei Blockaden: Label `blocked` setzen und Kommentar mit Problembeschreibung
"#,
        plankton_url = plankton_url,
    );

    (
        [
            (
                axum::http::header::CONTENT_TYPE,
                "text/markdown; charset=utf-8",
            ),
            (
                axum::http::header::CONTENT_DISPOSITION,
                "attachment; filename=\"SKILL.md\"",
            ),
        ],
        content,
    )
}

fn generate_docs_html() -> String {
    let tools = all_tools();
    let tool_rows: String = tools
        .iter()
        .map(|t| {
            let roles = t
                .roles
                .map(|r| r.join(", "))
                .unwrap_or_else(|| "all".to_string());
            format!(
                "<tr><td><code>{}</code></td><td>{}</td><td>{}</td></tr>",
                t.name, t.description, roles
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Plankton API Docs</title>
<style>
body {{ font-family: monospace; background: #0e0e10; color: #e2e2e8; max-width: 900px; margin: 0 auto; padding: 20px; line-height: 1.6; }}
h1 {{ color: #7c6af7; }} h2 {{ color: #90CAF9; border-bottom: 1px solid #2e2e38; padding-bottom: 4px; }}
code {{ background: #222228; padding: 2px 6px; border-radius: 3px; color: #CE93D8; }}
pre {{ background: #18181c; border: 1px solid #2e2e38; border-radius: 6px; padding: 12px; overflow-x: auto; }}
table {{ width: 100%; border-collapse: collapse; margin: 12px 0; }}
th, td {{ text-align: left; padding: 6px 10px; border: 1px solid #2e2e38; }}
th {{ background: #222228; color: #90CAF9; }}
a {{ color: #7c6af7; }}
</style>
</head>
<body>
<h1>Plankton API Documentation</h1>
<p>Kanban board with MCP (Model Context Protocol) support for LLM agents.</p>

<h2>Authentication</h2>
<p>All <code>/api/*</code> and <code>/mcp/*</code> endpoints require authentication.</p>
<ul>
<li><strong>Human users:</strong> JWT cookie set via <code>POST /auth/login</code></li>
<li><strong>LLM agents:</strong> Bearer token via <code>Authorization: Bearer plk_...</code></li>
</ul>
<pre>
POST /auth/login          {{"username":"...","password":"..."}}  → sets HttpOnly cookie
POST /auth/logout                                              → clears cookie
GET  /auth/me                                                  → current user info
POST /auth/change-password {{"old_password":"...","new_password":"..."}}
</pre>

<h2>Token Setup (for Agents)</h2>
<ol>
<li>Login as admin at <code>/auth/login</code></li>
<li>Create token: <code>POST /api/admin/tokens</code> with <code>{{"name":"my-agent","role":"developer"}}</code></li>
<li>Copy the <code>token</code> field from response (shown once!)</li>
<li>Use: <code>Authorization: Bearer plk_...</code> in all requests</li>
</ol>
<p>Available roles: <code>manager</code>, <code>developer</code>, <code>tester</code>, <code>admin</code></p>

<h2>REST API Endpoints</h2>
<pre>
GET    /api/projects                           → list projects
POST   /api/projects                           → create project
GET    /api/projects/:id                       → get project
PUT    /api/projects/:id                       → update project
DELETE /api/projects/:id?rev=...               → delete project
POST   /api/projects/:id/tasks                 → create task
PUT    /api/projects/:id/tasks/:tid            → update task
DELETE /api/projects/:id/tasks/:tid            → delete task
POST   /api/projects/:id/tasks/:tid/move       → move task {{"column_id":"...", "order": 0}}
POST   /api/projects/:id/columns               → create column
PUT    /api/projects/:id/columns/:cid          → update column
DELETE /api/projects/:id/columns/:cid          → delete column
GET    /api/projects/:id/events                → SSE event stream
</pre>

<h2>Admin Endpoints</h2>
<pre>
GET    /api/admin/users                        → list users
POST   /api/admin/users                        → create user
PUT    /api/admin/users/:uid                   → update user
DELETE /api/admin/users/:uid                   → delete user
PUT    /api/admin/users/:uid/password          → reset password
GET    /api/admin/tokens                       → list agent tokens
POST   /api/admin/tokens                       → create token
PUT    /api/admin/tokens/:tid                  → update token
DELETE /api/admin/tokens/:tid                  → delete token
</pre>

<h2>MCP Streamable HTTP Transport</h2>
<p>Protocol Version: <code>2025-03-26</code> – Session-basiert mit SSE.</p>
<pre>
POST   /mcp                    → JSON-RPC 2.0 (initialize erstellt Session)
GET    /mcp                    → SSE-Stream für Server-Notifications (Header: Mcp-Session-Id)
DELETE /mcp                    → Session beenden (Header: Mcp-Session-Id)
</pre>
<h3>Session-Flow</h3>
<pre>
1. POST /mcp  {{"method":"initialize"}}          → Response enthält Mcp-Session-Id Header
2. POST /mcp  {{"method":"tools/list"}}          → Header: Mcp-Session-Id: &lt;id&gt;
3. POST /mcp  {{"method":"tools/call",...}}       → Header: Mcp-Session-Id: &lt;id&gt;
4. GET  /mcp  (Header: Mcp-Session-Id)          → SSE-Stream (optional)
5. DELETE /mcp (Header: Mcp-Session-Id)          → Session beenden
</pre>
<h3>Beispiel</h3>
<pre>
// 1. Initialize → Session-ID aus Response-Header lesen
curl -D- -X POST /mcp -d '{{"jsonrpc":"2.0","method":"initialize","id":1}}'
// → Mcp-Session-Id: abc-123

// 2. Tools auflisten
curl -H "Mcp-Session-Id: abc-123" -X POST /mcp \
  -d '{{"jsonrpc":"2.0","method":"tools/list","id":2}}'

// 3. Tool aufrufen
curl -H "Mcp-Session-Id: abc-123" -X POST /mcp \
  -d '{{"jsonrpc":"2.0","method":"tools/call","params":{{"name":"list_projects","arguments":{{}}}},"id":3}}'
</pre>

<h2>Legacy MCP Endpoints</h2>
<pre>
GET  /mcp/tools                → list available tools
POST /mcp/call                 → {{"tool":"...","arguments":{{...}}}}
POST /mcp                      → JSON-RPC 2.0 (ohne Session, abwärtskompatibel)
</pre>

<h2>MCP Tools</h2>
<table>
<tr><th>Tool</th><th>Description</th><th>Roles</th></tr>
{tool_rows}
</table>

<h2>Workflow: Manager → Developer → Tester</h2>
<ol>
<li><strong>Manager</strong> creates tasks (<code>create_task</code>) and assigns them (<code>assign_task</code>)</li>
<li><strong>Developer</strong> picks up tasks (<code>get_assigned_tasks</code>), works on them (<code>update_task</code>, <code>add_log</code>), and submits for review (<code>submit_for_review</code>)</li>
<li><strong>Tester</strong> reviews tasks (<code>get_review_queue</code>), approves (<code>approve_task</code> → moves to Done) or rejects (<code>reject_task</code> → back to developer with comment)</li>
</ol>

<h2>Task Fields</h2>
<pre>
{{
  "id": "uuid",
  "title": "string",
  "description": "string",
  "column_id": "uuid (column/epic)",
  "previous_row": "uuid (previous column)",
  "labels": ["string"],
  "order": 0,
  "points": 0,
  "worker": "string (assigned developer)",
  "creator": "string (who created it)",
  "logs": ["string (audit trail)"],
  "comments": ["string"],
  "created_at": "ISO 8601",
  "updated_at": "ISO 8601"
}}
</pre>
</body>
</html>"#,
        tool_rows = tool_rows,
    )
}
