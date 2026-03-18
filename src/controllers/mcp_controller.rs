// Handler für MCP-Endpunkte (Legacy + JSON-RPC 2.0) und Docs.

use axum::{
    extract::State,
    Json,
};
use chrono::{Local, Utc};
use uuid::Uuid;

use crate::error::ApiError;
use crate::models::*;
use crate::services::*;
use crate::state::AppState;

/// Alle verfügbaren MCP-Tools mit optionaler Rollen-Einschränkung.
fn all_tools() -> Vec<ToolDef> {
    vec![
        ToolDef { name: "list_projects", description: "List all projects", roles: None },
        ToolDef { name: "get_project", description: "Get one project by id", roles: None },
        ToolDef { name: "summarize_board", description: "Summarize board column counts", roles: None },
        ToolDef { name: "create_project", description: "Create a new project", roles: Some(&["manager", "admin"]) },
        ToolDef { name: "list_epics", description: "List columns as epics with task counts", roles: Some(&["manager", "admin"]) },
        ToolDef { name: "create_task", description: "Create a task in a project", roles: Some(&["manager", "admin"]) },
        ToolDef { name: "assign_task", description: "Assign a worker to a task", roles: Some(&["manager", "admin"]) },
        ToolDef { name: "get_assigned_tasks", description: "Get tasks assigned to the caller", roles: Some(&["developer"]) },
        ToolDef { name: "update_task", description: "Update task title/description/labels", roles: Some(&["developer", "manager", "admin"]) },
        ToolDef { name: "add_log", description: "Append a log entry to a task", roles: Some(&["developer", "tester", "manager", "admin"]) },
        ToolDef { name: "submit_for_review", description: "Mark task as ready for review", roles: Some(&["developer"]) },
        ToolDef { name: "get_review_queue", description: "Get tasks waiting for review", roles: Some(&["tester"]) },
        ToolDef { name: "add_comment", description: "Add a comment to a task", roles: Some(&["tester", "developer", "manager", "admin"]) },
        ToolDef { name: "approve_task", description: "Approve and move task to Done", roles: Some(&["tester", "manager", "admin"]) },
        ToolDef { name: "reject_task", description: "Reject task and move back with comment", roles: Some(&["tester", "manager", "admin"]) },
        ToolDef { name: "move_task", description: "Move a task between columns", roles: Some(&["manager", "admin"]) },
        ToolDef { name: "delete_task", description: "Delete a task", roles: Some(&["manager", "admin"]) },
    ]
}

/// Tools nach Rolle filtern.
fn tools_for_role(role: &str) -> Vec<ToolDef> {
    all_tools()
        .into_iter()
        .filter(|t| match t.roles {
            None => true,
            Some(roles) => roles.contains(&role),
        })
        .collect()
}

/// Caller-Identität aus Headers auflösen (JWT oder Agent-Token).
async fn resolve_caller(headers: &axum::http::HeaderMap, state: &AppState) -> (String, String) {
    if let Some(t) = extract_token_from_headers(headers) {
        if let Ok(claims) = validate_jwt(&t, &state.jwt_secret) {
            return (claims.display_name, claims.role);
        }
    }
    if let Some(bearer) = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
    {
        if let Ok(agent_token) = state.store.get_token_by_value(bearer).await {
            return (agent_token.name, agent_token.role);
        }
    }
    ("anonymous".to_string(), String::new())
}

/// GET /mcp/tools – Verfügbare Tools auflisten.
pub async fn list_tools(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Json<Vec<ToolDef>> {
    let (_, role) = resolve_caller(&headers, &state).await;
    if role.is_empty() {
        Json(all_tools())
    } else {
        Json(tools_for_role(&role))
    }
}

/// POST /mcp/call – Ein Tool aufrufen (Legacy-Endpunkt).
pub async fn call_tool(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(call): Json<McpCall>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let (caller, _) = resolve_caller(&headers, &state).await;
    let out = execute_tool(&state, &call.tool, &call.arguments, &caller).await?;
    Ok(Json(out))
}

/// POST /mcp – JSON-RPC 2.0 MCP-Endpunkt.
pub async fn mcp_jsonrpc(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> Json<JsonRpcResponse> {
    let (caller, caller_role) = resolve_caller(&headers, &state).await;
    let rpc: JsonRpcRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            return Json(JsonRpcResponse {
                jsonrpc: "2.0".into(),
                result: None,
                error: Some(JsonRpcError { code: -32700, message: format!("Parse error: {e}") }),
                id: serde_json::Value::Null,
            })
        }
    };
    let id = rpc.id.clone().unwrap_or(serde_json::Value::Null);

    match rpc.method.as_str() {
        "initialize" => Json(JsonRpcResponse {
            jsonrpc: "2.0".into(),
            result: Some(serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "plankton-mcp", "version": "0.1.0" }
            })),
            error: None,
            id,
        }),
        "initialized" | "notifications/initialized" => Json(JsonRpcResponse {
            jsonrpc: "2.0".into(),
            result: Some(serde_json::json!({})),
            error: None,
            id,
        }),
        "tools/list" => {
            let tools = if caller_role.is_empty() {
                all_tools()
            } else {
                tools_for_role(&caller_role)
            };
            let tool_list: Vec<_> = tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "name": t.name,
                        "description": t.description,
                        "inputSchema": { "type": "object" }
                    })
                })
                .collect();
            Json(JsonRpcResponse {
                jsonrpc: "2.0".into(),
                result: Some(serde_json::json!({ "tools": tool_list })),
                error: None,
                id,
            })
        }
        "tools/call" => {
            let tool_name = rpc.params["name"]
                .as_str()
                .unwrap_or("");
            let arguments = rpc.params.get("arguments")
                .cloned()
                .unwrap_or(serde_json::json!({}));
            match execute_tool(&state, tool_name, &arguments, &caller).await {
                Ok(result) => Json(JsonRpcResponse {
                    jsonrpc: "2.0".into(),
                    result: Some(serde_json::json!({
                        "content": [{ "type": "text", "text": serde_json::to_string_pretty(&result).unwrap_or_default() }]
                    })),
                    error: None,
                    id,
                }),
                Err(e) => Json(JsonRpcResponse {
                    jsonrpc: "2.0".into(),
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32000,
                        message: format!("{e:?}"),
                    }),
                    id,
                }),
            }
        }
        _ => Json(JsonRpcResponse {
            jsonrpc: "2.0".into(),
            result: None,
            error: Some(JsonRpcError {
                code: -32601,
                message: format!("Method not found: {}", rpc.method),
            }),
            id,
        }),
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
                ..Task::default()
            };
            project.tasks.push(task);
            let updated = state.store.put_project(project).await?;
            publish_update(state, project_id).await;
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
                task.updated_at = Utc::now().to_rfc3339();
            }
            let _updated = state.store.put_project(project).await?;
            publish_update(state, project_id).await;
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
                task.updated_at = Utc::now().to_rfc3339();
                let log = format!(
                    "[{}] {} moved from {} to {}",
                    caller,
                    Local::now().format("%Y-%m-%d %H:%M"),
                    old_name,
                    new_name
                );
                task.logs.push(log);
            }
            let updated = state.store.put_project(project).await?;
            publish_update(state, project_id).await;
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
            project.tasks.retain(|t| t.id != task_id);
            let updated = state.store.put_project(project).await?;
            publish_update(state, project_id).await;
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
                task.logs.push(format!(
                    "[{}] {} assigned to {}",
                    caller,
                    Local::now().format("%Y-%m-%d %H:%M"),
                    worker
                ));
            } else {
                return Err(ApiError::NotFound("Task not found".into()));
            }
            let _updated = state.store.put_project(project).await?;
            publish_update(state, project_id).await;
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
                task.logs.push(format!(
                    "[{}] {} {}",
                    caller,
                    Local::now().format("%Y-%m-%d %H:%M"),
                    message
                ));
                task.updated_at = Utc::now().to_rfc3339();
            } else {
                return Err(ApiError::NotFound("Task not found".into()));
            }
            state.store.put_project(project).await?;
            publish_update(state, project_id).await;
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
                task.logs.push(format!(
                    "[{}] {} submitted for review",
                    caller,
                    Local::now().format("%Y-%m-%d %H:%M")
                ));
            } else {
                return Err(ApiError::NotFound("Task not found".into()));
            }
            state.store.put_project(project).await?;
            publish_update(state, project_id).await;
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
                task.comments.push(format!("[{}] {}", caller, text));
                task.updated_at = Utc::now().to_rfc3339();
            } else {
                return Err(ApiError::NotFound("Task not found".into()));
            }
            state.store.put_project(project).await?;
            publish_update(state, project_id).await;
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
                task.logs.push(format!(
                    "[{}] {} approved",
                    caller,
                    Local::now().format("%Y-%m-%d %H:%M")
                ));
            } else {
                return Err(ApiError::NotFound("Task not found".into()));
            }
            state.store.put_project(project).await?;
            publish_update(state, project_id).await;
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
                task.comments.push(format!("[{}] {}", caller, comment));
                task.logs.push(format!(
                    "[{}] {} rejected: {}",
                    caller,
                    Local::now().format("%Y-%m-%d %H:%M"),
                    comment
                ));
            } else {
                return Err(ApiError::NotFound("Task not found".into()));
            }
            state.store.put_project(project).await?;
            publish_update(state, project_id).await;
            Ok(serde_json::json!({"ok": true, "task_id": task_id}))
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

- **Server:** {plankton_url}
- **Dokumentation:** {plankton_url}/docs

## Secrets laden

Lies zuerst die Datei `plankton-secrets.md` aus einem der folgenden Orte (erster Treffer gewinnt):

1. `~/.claude/plankton-secrets.md` (persönlich, empfohlen)
2. `.claude/plankton-secrets.md` (projektlokal)

Die Secrets-Datei enthält Agent-Tokens und die Server-URL.
Generiere sie in der Plankton-Oberfläche unter **Projekt-Menü → Prompts → Claude Code Skill**.

## API-Aufrufe

Alle Tool-Aufrufe gehen an `POST {plankton_url}/mcp` als JSON-RPC 2.0.
Verwende den Token aus der Secrets-Datei als Bearer-Token.

### Aufruf-Muster

```bash
curl -s -X POST {plankton_url}/mcp \
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
- Beispiel: `{{"name":"create_task","arguments":{{"project_id":"ID","title":"Feature X","description":"Beschreibung","column_id":"SPALTE","labels":["feature"],"points":5}}}}`

**move_task** – Task in andere Spalte verschieben
- Parameter: `project_id`, `task_id`, `column_id` (alle string, required)

**assign_task** – Worker einem Task zuweisen
- Parameter: `project_id`, `task_id`, `worker` (alle string, required)

**delete_task** – Task löschen
- Parameter: `project_id`, `task_id` (beide string, required)

### Developer Tools

**get_assigned_tasks** – Dem Aufrufer zugewiesene Tasks
- Parameter: `project_id` (string, required)

**update_task** – Task bearbeiten (Titel, Beschreibung, Labels, Worker, Points)
- Parameter:
  - `project_id` (string, required)
  - `task_id` (string, required)
  - `title` (string, optional)
  - `description` (string, optional)
  - `labels` (string[], optional)
  - `worker` (string, optional)
  - `points` (number, optional)

**add_log** – Log-Eintrag zu einem Task hinzufügen
- Parameter: `project_id`, `task_id`, `message` (alle string, required)

**submit_for_review** – Task zur Review einreichen (setzt Label "review")
- Parameter: `project_id`, `task_id` (beide string, required)

### Tester Tools

**get_review_queue** – Tasks mit Label "review" auflisten
- Parameter: `project_id` (string, required)

**add_comment** – Kommentar zu einem Task hinzufügen
- Parameter: `project_id`, `task_id`, `text` (alle string, required)

**approve_task** – Task abnehmen (verschiebt nach "Done", entfernt "review"-Label)
- Parameter: `project_id`, `task_id` (beide string, required)

**reject_task** – Task zurückweisen (verschiebt zurück, entfernt "review"-Label)
- Parameter: `project_id`, `task_id` (beide string, required), `comment` (string, optional)

## Typischer Workflow

1. `list_projects` → Projekt-ID finden
2. `get_project` → Spalten-IDs und Tasks sehen
3. `create_task` → Neuen Task anlegen
4. `move_task` → Task in "In Progress" verschieben
5. `add_log` → Fortschritt dokumentieren
6. `submit_for_review` → Zur Review einreichen
7. `approve_task` / `reject_task` → Review abschließen

## Vollständiges Beispiel

```bash
# Token aus secrets.md laden
TOKEN="plk_xxx..."

# Projekte auflisten
curl -s -X POST {plankton_url}/mcp \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{{"jsonrpc":"2.0","method":"tools/call","params":{{"name":"list_projects","arguments":{{}}}},"id":1}}'

# Task erstellen
curl -s -X POST {plankton_url}/mcp \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{{"jsonrpc":"2.0","method":"tools/call","params":{{"name":"create_task","arguments":{{"project_id":"PROJ_ID","title":"Bug fixen","description":"Details...","labels":["bug"],"points":3}}}},"id":2}}'
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

<h2>MCP Protocol (JSON-RPC 2.0)</h2>
<p>Endpoint: <code>POST /mcp</code></p>
<pre>
// Initialize
{{"jsonrpc":"2.0","method":"initialize","id":1}}

// List tools
{{"jsonrpc":"2.0","method":"tools/list","id":2}}

// Call a tool
{{"jsonrpc":"2.0","method":"tools/call","params":{{"name":"list_projects","arguments":{{}}}},"id":3}}
</pre>

<h2>Legacy MCP Endpoints</h2>
<pre>
GET  /mcp/tools                → list available tools
POST /mcp/call                 → {{"tool":"...","arguments":{{...}}}}
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
