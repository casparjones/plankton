// Handler für MCP-Endpunkte (Legacy + Streamable HTTP Transport) und Docs.

use axum::{
    extract::State,
    response::{sse::Event, Sse},
    Json,
};
use chrono::{Local, Utc};
use futures::{stream, Stream, StreamExt as _};
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::error::ApiError;
use crate::models::*;
use crate::services::*;
use crate::state::{AppState, McpSession};

/// Alle verfügbaren MCP-Tools mit optionaler Rollen-Einschränkung.
fn all_tools() -> Vec<ToolDef> {
    vec![
        ToolDef { name: "list_projects", description: "List all projects", roles: None, schema: None },
        ToolDef { name: "get_project", description: "Get one project by id", roles: None, schema: Some(|| serde_json::json!({
            "type": "object", "required": ["id"],
            "properties": { "id": { "type": "string", "description": "Project ID" } }
        })) },
        ToolDef { name: "summarize_board", description: "Summarize board column counts", roles: None, schema: Some(|| serde_json::json!({
            "type": "object", "required": ["project_id"],
            "properties": { "project_id": { "type": "string", "description": "Project ID" } }
        })) },
        ToolDef { name: "create_project", description: "Create a new project", roles: Some(&["manager", "admin"]), schema: Some(|| serde_json::json!({
            "type": "object",
            "properties": { "title": { "type": "string", "description": "Project title" } }
        })) },
        ToolDef { name: "list_epics", description: "List columns as epics with task counts", roles: Some(&["manager", "admin"]), schema: Some(|| serde_json::json!({
            "type": "object", "required": ["project_id"],
            "properties": { "project_id": { "type": "string" } }
        })) },
        ToolDef { name: "create_task", description: "Create a task in a project", roles: Some(&["manager", "admin"]), schema: Some(|| serde_json::json!({
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
        ToolDef { name: "assign_task", description: "Assign a worker to a task", roles: Some(&["manager", "admin"]), schema: Some(|| serde_json::json!({
            "type": "object", "required": ["project_id", "task_id", "worker"],
            "properties": { "project_id": { "type": "string" }, "task_id": { "type": "string" }, "worker": { "type": "string" } }
        })) },
        ToolDef { name: "get_assigned_tasks", description: "Get tasks assigned to the caller", roles: Some(&["developer"]), schema: Some(|| serde_json::json!({
            "type": "object", "required": ["project_id"],
            "properties": { "project_id": { "type": "string" } }
        })) },
        ToolDef { name: "update_task", description: "Update task title/description/labels", roles: Some(&["developer", "manager", "admin"]), schema: Some(|| serde_json::json!({
            "type": "object", "required": ["project_id", "task_id"],
            "properties": {
                "project_id": { "type": "string" }, "task_id": { "type": "string" },
                "title": { "type": "string" }, "description": { "type": "string" },
                "labels": { "type": "array", "items": { "type": "string" } },
                "worker": { "type": "string" }, "points": { "type": "number" },
                "task_type": { "type": "string", "enum": ["task", "epic", "job"] },
                "parent_id": { "type": "string" }
            }
        })) },
        ToolDef { name: "add_log", description: "Append a log entry to a task", roles: Some(&["developer", "tester", "manager", "admin"]), schema: Some(|| serde_json::json!({
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
        ToolDef { name: "add_comment", description: "Add a comment to a task", roles: Some(&["tester", "developer", "manager", "admin"]), schema: Some(|| serde_json::json!({
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
        ToolDef { name: "move_task", description: "Move a task between columns", roles: Some(&["manager", "admin"]), schema: Some(|| serde_json::json!({
            "type": "object", "required": ["project_id", "task_id", "column_id"],
            "properties": { "project_id": { "type": "string" }, "task_id": { "type": "string" }, "column_id": { "type": "string" }, "order": { "type": "number" } }
        })) },
        ToolDef { name: "delete_task", description: "Delete a task", roles: Some(&["manager", "admin"]), schema: Some(|| serde_json::json!({
            "type": "object", "required": ["project_id", "task_id"],
            "properties": { "project_id": { "type": "string" }, "task_id": { "type": "string" } }
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
async fn resolve_caller(headers: &axum::http::HeaderMap, state: &AppState) -> Result<(String, String), ApiError> {
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
    use axum::response::IntoResponse;
    use axum::http::{StatusCode, header};
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
    ).into_response()
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
    session_id: &Option<String>,
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
            if is_notification { return None; }
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
                    let schema = t.schema.map(|f| f()).unwrap_or_else(|| serde_json::json!({"type": "object", "properties": {}}));
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
            let tool_name = rpc.params["name"]
                .as_str()
                .unwrap_or("");
            let arguments = rpc.params.get("arguments")
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

    if is_notification { None } else { Some(response) }
}

/// POST /mcp – JSON-RPC 2.0 MCP-Endpunkt mit Streamable HTTP Transport.
/// Unterstützt sowohl JSON- als auch SSE-Antworten (je nach Accept-Header).
/// Erstellt bei "initialize" eine Session und gibt Mcp-Session-Id Header zurück.
pub async fn mcp_jsonrpc(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    use axum::http::{header, StatusCode};

    let use_sse = wants_sse(&headers);
    let auth_result = resolve_caller(&headers, &state).await;

    // Host/Scheme für WWW-Authenticate Header
    let scheme = headers.get("x-forwarded-proto").and_then(|v| v.to_str().ok()).unwrap_or("http");
    let host = headers.get("host").and_then(|v| v.to_str().ok()).unwrap_or("localhost");

    // Session-ID aus Header
    let session_id = headers
        .get("mcp-session-id")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    // Auth-Check VOR Body-Parsing: Kein Token UND keine gültige Session → 401
    if auth_result.is_err() {
        let has_valid_session = if let Some(ref sid) = session_id {
            let sessions = state.mcp_sessions.lock().await;
            sessions.get(sid).map(|s| !s.caller.is_empty() && !s.role.is_empty()).unwrap_or(false)
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
                error: Some(JsonRpcError { code: -32700, message: format!("Parse error: {e}") }),
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
                    error: Some(JsonRpcError { code: -32600, message: format!("Invalid batch: {e}") }),
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
                    error: Some(JsonRpcError { code: -32600, message: format!("Invalid request: {e}") }),
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
    if !has_init {
        match &session_id {
            Some(sid) => {
                let sessions = state.mcp_sessions.lock().await;
                if !sessions.contains_key(sid) {
                    let resp = JsonRpcResponse {
                        jsonrpc: "2.0".into(),
                        result: None,
                        error: Some(JsonRpcError { code: -32001, message: "Invalid or expired session".into() }),
                        id: serde_json::Value::Null,
                    };
                    return (StatusCode::NOT_FOUND, Json(resp)).into_response();
                }
            }
            None => {
                // Kein Session-Header und kein initialize → Session erforderlich
                let resp = JsonRpcResponse {
                    jsonrpc: "2.0".into(),
                    result: None,
                    error: Some(JsonRpcError { code: -32002, message: "Mcp-Session-Id header required".into() }),
                    id: serde_json::Value::Null,
                };
                return (StatusCode::BAD_REQUEST, Json(resp)).into_response();
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
        state.mcp_sessions.lock().await.insert(new_session_id.clone(), session);

        // protocolVersion vom Client übernehmen (Kompatibilität mit 2024-11-05 und 2025-03-26)
        let client_version = init_rpc.params.get("protocolVersion")
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
            [
                (header::HeaderName::from_static("mcp-session-id"), new_session_id),
            ],
            Json(resp),
        ).into_response();
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
        if let Some(resp) = handle_single_rpc(&state, rpc, &caller, &caller_role, &session_id).await {
            responses.push(resp);
        }
    }

    // Nur Notifications (keine Responses) → sofort 202 Accepted, egal ob SSE oder nicht
    if responses.is_empty() {
        let sid_header = session_id.unwrap_or_default();
        return (
            StatusCode::ACCEPTED,
            [(header::HeaderName::from_static("mcp-session-id"), sid_header)],
        ).into_response();
    }

    // SSE-Modus: Antworten als SSE-Events streamen (nur initiale Events, kein long-lived Stream)
    // Long-lived SSE-Streams gehören zu GET /mcp, nicht zu POST Request-Response-Calls
    if use_sse {
        let initial_events: Vec<Result<Event, std::convert::Infallible>> = responses
            .into_iter()
            .filter_map(|r| {
                serde_json::to_string(&r).ok().map(|json| {
                    Ok(Event::default().event("message").data(json))
                })
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
            [(header::HeaderName::from_static("mcp-session-id"), sid_header)],
            Json(responses.into_iter().next().unwrap()),
        ).into_response()
    } else if responses.is_empty() {
        // Nur Notifications – kein Body, 202 Accepted
        (
            StatusCode::ACCEPTED,
            [(header::HeaderName::from_static("mcp-session-id"), sid_header)],
        ).into_response()
    } else {
        (
            [(header::HeaderName::from_static("mcp-session-id"), sid_header)],
            Json(responses),
        ).into_response()
    }
}

/// GET /mcp – SSE-Stream für Server-initiierte Nachrichten (Streamable HTTP Transport).
pub async fn mcp_sse_stream(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    use axum::http::StatusCode;

    // Auth prüfen: 401 mit WWW-Authenticate wenn kein gültiger Token
    if resolve_caller(&headers, &state).await.is_err() {
        let scheme = headers.get("x-forwarded-proto").and_then(|v| v.to_str().ok()).unwrap_or("http");
        let host = headers.get("host").and_then(|v| v.to_str().ok()).unwrap_or("localhost");
        return unauthorized_response(host, scheme);
    }

    let session_id = match headers
        .get("mcp-session-id")
        .and_then(|v| v.to_str().ok())
    {
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
            Ok(msg) => Some((Ok::<_, std::convert::Infallible>(Event::default().data(msg)), rx)),
            Err(broadcast::error::RecvError::Lagged(_)) => {
                Some((Ok::<_, std::convert::Infallible>(Event::default().event("heartbeat").data("ping")), rx))
            }
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

    let session_id = match headers
        .get("mcp-session-id")
        .and_then(|v| v.to_str().ok())
    {
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

/// Zentraler Tool-Executor für Legacy und JSON-RPC MCP.
async fn execute_tool(
    state: &AppState,
    tool: &str,
    args: &serde_json::Value,
    caller: &str,
) -> Result<serde_json::Value, ApiError> {
    match tool {
        "list_projects" => Ok(serde_json::to_value(state.store.list_projects().await?)?),
        "get_project" => {
            let id = args["id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("id missing".into()))?;
            Ok(serde_json::to_value(state.store.get_project(id).await?)?)
        }
        "create_project" => {
            let title = args["title"]
                .as_str()
                .unwrap_or("Untitled Project");
            let project = default_project(title.to_string());
            Ok(serde_json::to_value(state.store.create_project(project).await?)?)
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
                    .unwrap_or(
                        project.columns.first().map(|c| c.id.as_str()).unwrap_or(""),
                    )
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
            let updated = state.store.put_project(project).await?;
            publish_event(state, project_id, "task_created", serde_json::to_value(&task)?).await;
            Ok(serde_json::to_value(updated)?)
        }
        "update_task" => {
            let project_id = args["project_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("project_id missing".into()))?;
            let task_id = args["task_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("task_id missing".into()))?;
            let mut project = state.store.get_project(project_id).await?;
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
            let _updated = state.store.put_project(project).await?;
            if let Some(t) = task_data {
                publish_event(state, project_id, "task_updated", serde_json::to_value(&t)?).await;
            }
            Ok(serde_json::to_value(_updated)?)
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
            let mut project = state.store.get_project(project_id).await?;
            let col_name = |cid: &str| -> String {
                project
                    .columns
                    .iter()
                    .find(|c| c.id == cid)
                    .map(|c| c.title.clone())
                    .unwrap_or_else(|| cid.to_string())
            };
            if let Some(task) = project.tasks.iter_mut().find(|t| t.id == task_id) {
                let old_name = col_name(&task.column_id);
                let new_name = col_name(column_id);
                task.previous_row = task.column_id.clone();
                task.column_id = column_id.to_string();
                if let Some(order) = args["order"].as_i64() {
                    task.order = order as i32;
                }
                task.updated_at = Utc::now().to_rfc3339();
                task.logs.push(log_entry(&caller, &format!("→ {}", new_name)));
            }
            let task_data = project.tasks.iter().find(|t| t.id == task_id).cloned();
            let updated = state.store.put_project(project).await?;
            if let Some(t) = task_data {
                publish_event(state, project_id, "task_moved", serde_json::to_value(&t)?).await;
            }
            Ok(serde_json::to_value(updated)?)
        }
        "delete_task" => {
            let project_id = args["project_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("project_id missing".into()))?;
            let task_id = args["task_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("task_id missing".into()))?;
            let mut project = state.store.get_project(project_id).await?;
            // Relationen aufräumen
            for task in &mut project.tasks {
                task.blocks.retain(|id| id != task_id);
                task.blocked_by.retain(|id| id != task_id);
                task.subtask_ids.retain(|id| id != task_id);
                if task.parent_id == task_id { task.parent_id.clear(); }
            }
            project.tasks.retain(|t| t.id != task_id);
            let updated = state.store.put_project(project).await?;
            publish_event(state, project_id, "task_deleted", serde_json::json!({ "task_id": task_id })).await;
            Ok(serde_json::to_value(updated)?)
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
            let mut visible_cols: Vec<_> =
                project.columns.iter().filter(|c| !c.hidden).collect();
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
            let mut project = state.store.get_project(project_id).await?;
            if let Some(task) = project.tasks.iter_mut().find(|t| t.id == task_id) {
                task.worker = worker.to_string();
                task.updated_at = Utc::now().to_rfc3339();
                task.logs.push(log_entry(&caller, &format!("assigned → {}", worker)));
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
                task.logs.push(log_entry(&caller, message));
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
        "submit_for_review" => {
            let project_id = args["project_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("project_id missing".into()))?;
            let task_id = args["task_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("task_id missing".into()))?;
            let mut project = state.store.get_project(project_id).await?;
            if let Some(task) = project.tasks.iter_mut().find(|t| t.id == task_id) {
                if !task.labels.contains(&"review".to_string()) {
                    task.labels.push("review".to_string());
                }
                task.updated_at = Utc::now().to_rfc3339();
                task.logs.push(log_entry(&caller, "submitted for review"));
            } else {
                return Err(ApiError::NotFound("Task not found".into()));
            }
            let task_data = project.tasks.iter().find(|t| t.id == task_id).cloned();
            state.store.put_project(project).await?;
            if let Some(t) = task_data {
                publish_event(state, project_id, "task_updated", serde_json::to_value(&t)?).await;
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
                task.comments.push(log_entry(&caller, text));
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
                }
                task.updated_at = Utc::now().to_rfc3339();
                task.logs.push(log_entry(&caller, "✓ approved"));
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
            let comment = args["comment"]
                .as_str()
                .unwrap_or("Rejected");
            let mut project = state.store.get_project(project_id).await?;
            if let Some(task) = project.tasks.iter_mut().find(|t| t.id == task_id) {
                task.labels.retain(|l| l != "review");
                if !task.previous_row.is_empty() {
                    let prev = task.previous_row.clone();
                    task.column_id = prev;
                }
                task.updated_at = Utc::now().to_rfc3339();
                task.comments.push(log_entry(&caller, &comment));
                task.logs.push(log_entry(&caller, &format!("✗ rejected: {}", comment)));
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
            let done_col = project.columns.iter().find(|c| c.title == "Done").map(|c| c.id.as_str());
            let subtasks: Vec<_> = project
                .tasks
                .iter()
                .filter(|t| t.parent_id == parent_id)
                .map(|t| {
                    let is_done = done_col.map(|d| t.column_id == d).unwrap_or(false);
                    let col_name = project.columns.iter().find(|c| c.id == t.column_id).map(|c| c.title.as_str()).unwrap_or("?");
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
                        if !t.blocks.contains(&to_id.to_string()) { t.blocks.push(to_id.to_string()); }
                    }
                    if let Some(t) = project.tasks.iter_mut().find(|t| t.id == to_id) {
                        if !t.blocked_by.contains(&from_id.to_string()) { t.blocked_by.push(from_id.to_string()); }
                    }
                }
                "subtask" => {
                    // from = parent (epic), to = child (subtask)
                    if let Some(t) = project.tasks.iter_mut().find(|t| t.id == from_id) {
                        if !t.subtask_ids.contains(&to_id.to_string()) { t.subtask_ids.push(to_id.to_string()); }
                    }
                    if let Some(t) = project.tasks.iter_mut().find(|t| t.id == to_id) {
                        t.parent_id = from_id.to_string();
                    }
                }
                _ => return Err(ApiError::BadRequest(format!("unknown relation: {relation} (use blocks|subtask)"))),
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
                _ => return Err(ApiError::BadRequest(format!("unknown relation: {relation}"))),
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
            let task_ids = args["task_ids"]
                .as_array()
                .ok_or_else(|| ApiError::BadRequest("task_ids missing (array of task IDs in desired order)".into()))?;
            let mut project = state.store.get_project(project_id).await?;
            let mut reordered = 0;
            for (i, tid_val) in task_ids.iter().enumerate() {
                if let Some(tid) = tid_val.as_str() {
                    if let Some(task) = project.tasks.iter_mut().find(|t| t.id == tid && t.column_id == column_id) {
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
        _ => Err(ApiError::BadRequest(format!("unknown tool: {tool}"))),
    }
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
            (axum::http::header::CONTENT_TYPE, "text/markdown; charset=utf-8"),
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
