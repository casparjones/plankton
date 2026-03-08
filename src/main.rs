// ============================================================
// Plankton – Kanban-Backend (Axum + CouchDB oder File-Store)
// ============================================================
// Dieses Modul stellt eine REST-API für ein Kanban-Board bereit.
// Als Storage-Backend kann entweder CouchDB (via COUCHDB_URI)
// oder ein einfacher JSON-File-Store (./data/projects) verwendet werden.
// Zusätzlich gibt es einen minimalen MCP-Endpunkt (/mcp/*) für
// Tool-basierte KI-Zugriffe sowie SSE-Events pro Projekt.
// ============================================================

use std::{collections::HashMap, net::SocketAddr, path::PathBuf, sync::Arc};

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{sse::Event, IntoResponse, Sse},
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::Utc;
use futures::{stream, Stream};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, Mutex};
use tower_http::{cors::CorsLayer, services::ServeDir};
use tracing::info;
use uuid::Uuid;

// ------------------------------------------------------------------
// App-State
// ------------------------------------------------------------------

/// Zentraler Anwendungs-State, der von Axum in alle Handler injiziert wird.
#[derive(Clone)]
struct AppState {
    /// Abstrahiertes Storage-Backend (CouchDB oder File).
    store: DataStore,
    /// Pro Projekt-ID ein Broadcast-Sender für SSE-Events.
    events: Arc<Mutex<HashMap<String, broadcast::Sender<String>>>>,
}

// ------------------------------------------------------------------
// Storage-Backends
// ------------------------------------------------------------------

/// Enum, das CouchDB und den lokalen File-Store vereint.
/// Alle Methoden werden über `DataStore::*` aufgerufen und delegieren
/// intern an das passende Backend.
#[derive(Clone)]
enum DataStore {
    Couch(CouchDb),
    File(FileStore),
}

/// HTTP-Client-Wrapper für CouchDB.
#[derive(Clone)]
struct CouchDb {
    client: Client,
    /// Basis-URL, z.B. "http://admin:password@localhost:5984"
    base_url: String,
    /// Name der Datenbank, z.B. "plankton"
    db: String,
}

/// Lokaler File-Store: Jedes Projekt wird als `<id>.json` in `root` gespeichert.
#[derive(Clone)]
struct FileStore {
    root: PathBuf,
}

// ------------------------------------------------------------------
// Datenmodelle
// ------------------------------------------------------------------

/// Repräsentiert ein vollständiges Kanban-Projekt als flaches Dokument.
/// Sowohl CouchDB-Felder (`_id`, `_rev`) als auch die eigentlichen Daten
/// (Spalten, Nutzer, Aufgaben) sind enthalten.
#[derive(Debug, Serialize, Deserialize, Clone)]
struct ProjectDoc {
    #[serde(rename = "_id")]
    id: String,
    /// Revisions-Token – wird von CouchDB benötigt und im FileStore simuliert.
    #[serde(rename = "_rev", skip_serializing_if = "Option::is_none")]
    rev: Option<String>,
    title: String,
    columns: Vec<Column>,
    users: Vec<User>,
    tasks: Vec<Task>,
}

/// Eine Spalte im Kanban-Board.
#[derive(Debug, Serialize, Deserialize, Clone)]
struct Column {
    id: String,
    title: String,
    /// Reihenfolge der Spalte (aufsteigend).
    order: i32,
    /// Hex-Farbcode, z.B. "#90CAF9".
    color: String,
}

/// Ein Teammitglied, das Aufgaben zugewiesen bekommen kann.
#[derive(Debug, Serialize, Deserialize, Clone)]
struct User {
    id: String,
    name: String,
    /// URL oder Initialen-Kürzel für den Avatar.
    avatar: String,
    role: String,
}

/// Eine einzelne Aufgabe (Karte) im Board.
#[derive(Debug, Serialize, Deserialize, Clone)]
struct Task {
    id: String,
    title: String,
    description: String,
    /// ID der Spalte, in der sich die Aufgabe befindet.
    column_id: String,
    assignee_ids: Vec<String>,
    labels: Vec<String>,
    /// Reihenfolge innerhalb der Spalte.
    order: i32,
    created_at: String,
    updated_at: String,
}

// ------------------------------------------------------------------
// Request/Response-Hilfstypen
// ------------------------------------------------------------------

/// Query-Parameter für DELETE /projects/:id – CouchDB erfordert die Rev.
#[derive(Debug, Deserialize)]
struct DeleteQuery {
    rev: String,
}

/// Body für POST /projects/:id/tasks/:task_id/move
#[derive(Debug, Deserialize)]
struct MoveTaskRequest {
    column_id: String,
    order: Option<i32>,
}

/// Body für POST /mcp/call
#[derive(Debug, Deserialize)]
struct McpCall {
    tool: String,
    arguments: serde_json::Value,
}

/// Tool-Beschreibung für GET /mcp/tools
#[derive(Debug, Serialize)]
struct ToolDef {
    name: &'static str,
    description: &'static str,
}

// ------------------------------------------------------------------
// main
// ------------------------------------------------------------------

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // CouchDB-URL kann über COUCHDB_URI oder COUCHDB_URL gesetzt werden.
    let couch_uri = std::env::var("COUCHDB_URI")
        .ok()
        .or_else(|| std::env::var("COUCHDB_URL").ok());
    let db = std::env::var("COUCHDB_DB").unwrap_or_else(|_| "plankton".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());

    // Backend-Auswahl: CouchDB wenn URI gesetzt, sonst File-Store.
    let store = if let Some(base_url) = couch_uri {
        let couch = CouchDb {
            client: Client::new(),
            base_url,
            db,
        };
        couch.ensure_db().await?;
        info!("using CouchDB backend");
        DataStore::Couch(couch)
    } else {
        let files = FileStore {
            root: PathBuf::from("data/projects"),
        };
        files.ensure_db().await?;
        info!("COUCHDB_URI is not set; using local file backend in ./data/projects");
        DataStore::File(files)
    };

    let state = AppState {
        store,
        events: Arc::new(Mutex::new(HashMap::new())),
    };

    // Router: REST-API + MCP-Endpunkte + Statische Dateien.
    let app = Router::new()
        .route("/api/projects", get(list_projects).post(create_project))
        .route(
            "/api/projects/:id",
            get(get_project).put(update_project).delete(delete_project),
        )
        .route("/api/projects/:id/tasks", post(create_task))
        .route(
            "/api/projects/:id/tasks/:task_id",
            put(update_task).delete(delete_task),
        )
        .route("/api/projects/:id/tasks/:task_id/move", post(move_task))
        .route("/api/projects/:id/columns", post(create_column))
        .route(
            "/api/projects/:id/columns/:column_id",
            put(update_column).delete(delete_column),
        )
        .route("/api/projects/:id/users", post(create_user))
        .route(
            "/api/projects/:id/users/:user_id",
            put(update_user).delete(delete_user),
        )
        .route("/api/projects/:id/events", get(project_events))
        .route("/mcp/tools", get(list_tools))
        .route("/mcp/call", post(call_tool))
        .nest_service(
            "/",
            ServeDir::new("static").append_index_html_on_directories(true),
        )
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr: SocketAddr = format!("0.0.0.0:{port}").parse()?;
    info!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

// ------------------------------------------------------------------
// Projekt-Handler
// ------------------------------------------------------------------

/// GET /api/projects – Alle Projekte auflisten.
async fn list_projects(State(state): State<AppState>) -> Result<Json<Vec<ProjectDoc>>, ApiError> {
    Ok(Json(state.store.list_projects().await?))
}

/// POST /api/projects – Neues Projekt anlegen.
async fn create_project(
    State(state): State<AppState>,
    Json(mut payload): Json<ProjectDoc>,
) -> Result<Json<ProjectDoc>, ApiError> {
    if payload.id.is_empty() {
        payload.id = Uuid::new_v4().to_string();
    }
    payload.rev = None;
    let created = state.store.create_project(payload).await?;
    publish_update(&state, &created.id).await;
    Ok(Json(created))
}

/// GET /api/projects/:id – Ein Projekt abrufen.
async fn get_project(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ProjectDoc>, ApiError> {
    Ok(Json(state.store.get_project(&id).await?))
}

/// PUT /api/projects/:id – Vollständiges Projekt ersetzen.
///
/// FIX: Das Projekt wird zunächst aus dem Store geladen, damit die aktuelle
/// Revisions-ID bekannt ist. Ohne diesen Schritt würde der FileStore
/// immer einen Conflict-Fehler melden, weil das eingehende Payload keine Rev trägt.
async fn update_project(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(mut payload): Json<ProjectDoc>,
) -> Result<Json<ProjectDoc>, ApiError> {
    // Sicherstellen, dass ID und Rev korrekt gesetzt sind.
    payload.id = id.clone();
    // Aktuelle Rev aus dem Store holen, damit der FileStore keinen Conflict wirft.
    let current = state.store.get_project(&id).await?;
    payload.rev = current.rev;
    let updated = state.store.put_project(payload).await?;
    publish_update(&state, &id).await;
    Ok(Json(updated))
}

/// DELETE /api/projects/:id?rev=<rev> – Projekt löschen.
async fn delete_project(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<DeleteQuery>,
) -> Result<StatusCode, ApiError> {
    state.store.delete_project(&id, &query.rev).await?;
    publish_update(&state, &id).await;
    Ok(StatusCode::NO_CONTENT)
}

// ------------------------------------------------------------------
// Task-Handler
// ------------------------------------------------------------------

/// POST /api/projects/:id/tasks – Neue Aufgabe anlegen.
async fn create_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(mut task): Json<Task>,
) -> Result<Json<ProjectDoc>, ApiError> {
    let mut project = state.store.get_project(&id).await?;
    if task.id.is_empty() {
        task.id = Uuid::new_v4().to_string();
    }
    let now = Utc::now().to_rfc3339();
    task.created_at = now.clone();
    task.updated_at = now;
    project.tasks.push(task);
    let updated = state.store.put_project(project).await?;
    publish_update(&state, &id).await;
    Ok(Json(updated))
}

/// PUT /api/projects/:id/tasks/:task_id – Aufgabe aktualisieren.
async fn update_task(
    State(state): State<AppState>,
    Path((id, task_id)): Path<(String, String)>,
    Json(task): Json<Task>,
) -> Result<Json<ProjectDoc>, ApiError> {
    let mut project = state.store.get_project(&id).await?;
    if let Some(existing) = project.tasks.iter_mut().find(|t| t.id == task_id) {
        *existing = Task {
            updated_at: Utc::now().to_rfc3339(),
            ..task
        };
    } else {
        return Err(ApiError::NotFound("Task not found".into()));
    }
    let updated = state.store.put_project(project).await?;
    publish_update(&state, &id).await;
    Ok(Json(updated))
}

/// DELETE /api/projects/:id/tasks/:task_id – Aufgabe löschen.
async fn delete_task(
    State(state): State<AppState>,
    Path((id, task_id)): Path<(String, String)>,
) -> Result<Json<ProjectDoc>, ApiError> {
    let mut project = state.store.get_project(&id).await?;
    project.tasks.retain(|t| t.id != task_id);
    let updated = state.store.put_project(project).await?;
    publish_update(&state, &id).await;
    Ok(Json(updated))
}

/// POST /api/projects/:id/tasks/:task_id/move – Aufgabe in eine andere Spalte verschieben.
async fn move_task(
    State(state): State<AppState>,
    Path((id, task_id)): Path<(String, String)>,
    Json(req): Json<MoveTaskRequest>,
) -> Result<Json<ProjectDoc>, ApiError> {
    let mut project = state.store.get_project(&id).await?;
    if let Some(task) = project.tasks.iter_mut().find(|t| t.id == task_id) {
        task.column_id = req.column_id;
        task.order = req.order.unwrap_or(task.order);
        task.updated_at = Utc::now().to_rfc3339();
    } else {
        return Err(ApiError::NotFound("Task not found".into()));
    }
    let updated = state.store.put_project(project).await?;
    publish_update(&state, &id).await;
    Ok(Json(updated))
}

// ------------------------------------------------------------------
// Spalten-Handler
// ------------------------------------------------------------------

/// POST /api/projects/:id/columns – Neue Spalte anlegen.
async fn create_column(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(mut column): Json<Column>,
) -> Result<Json<ProjectDoc>, ApiError> {
    let mut project = state.store.get_project(&id).await?;
    if column.id.is_empty() {
        column.id = Uuid::new_v4().to_string();
    }
    project.columns.push(column);
    let updated = state.store.put_project(project).await?;
    publish_update(&state, &id).await;
    Ok(Json(updated))
}

/// PUT /api/projects/:id/columns/:column_id – Spalte aktualisieren.
async fn update_column(
    State(state): State<AppState>,
    Path((id, column_id)): Path<(String, String)>,
    Json(column): Json<Column>,
) -> Result<Json<ProjectDoc>, ApiError> {
    let mut project = state.store.get_project(&id).await?;
    if let Some(existing) = project.columns.iter_mut().find(|c| c.id == column_id) {
        *existing = column;
    }
    let updated = state.store.put_project(project).await?;
    publish_update(&state, &id).await;
    Ok(Json(updated))
}

/// DELETE /api/projects/:id/columns/:column_id – Spalte und alle ihre Aufgaben löschen.
async fn delete_column(
    State(state): State<AppState>,
    Path((id, column_id)): Path<(String, String)>,
) -> Result<Json<ProjectDoc>, ApiError> {
    let mut project = state.store.get_project(&id).await?;
    project.columns.retain(|c| c.id != column_id);
    // Aufgaben der gelöschten Spalte ebenfalls entfernen.
    project.tasks.retain(|t| t.column_id != column_id);
    let updated = state.store.put_project(project).await?;
    publish_update(&state, &id).await;
    Ok(Json(updated))
}

// ------------------------------------------------------------------
// Nutzer-Handler
// ------------------------------------------------------------------

/// POST /api/projects/:id/users – Neuen Nutzer zum Projekt hinzufügen.
async fn create_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(mut user): Json<User>,
) -> Result<Json<ProjectDoc>, ApiError> {
    let mut project = state.store.get_project(&id).await?;
    if user.id.is_empty() {
        user.id = Uuid::new_v4().to_string();
    }
    project.users.push(user);
    let updated = state.store.put_project(project).await?;
    publish_update(&state, &id).await;
    Ok(Json(updated))
}

/// PUT /api/projects/:id/users/:user_id – Nutzer aktualisieren.
///
/// FIX: War `state.couch.get_project` / `state.couch.put_project` –
/// `AppState` hat kein Feld `couch`. Korrigiert zu `state.store.*`.
async fn update_user(
    State(state): State<AppState>,
    Path((id, user_id)): Path<(String, String)>,
    Json(user): Json<User>,
) -> Result<Json<ProjectDoc>, ApiError> {
    let mut project = state.store.get_project(&id).await?; // FIX: war state.couch
    if let Some(existing) = project.users.iter_mut().find(|u| u.id == user_id) {
        *existing = user;
    }
    let updated = state.store.put_project(project).await?; // FIX: war state.couch
    publish_update(&state, &id).await;
    Ok(Json(updated))
}

/// DELETE /api/projects/:id/users/:user_id – Nutzer aus dem Projekt entfernen.
///
/// FIX: War `state.couch.get_project` / `state.couch.put_project` –
/// korrigiert zu `state.store.*`. Außerdem werden Zuweisung-IDs in allen
/// Aufgaben bereinigt.
async fn delete_user(
    State(state): State<AppState>,
    Path((id, user_id)): Path<(String, String)>,
) -> Result<Json<ProjectDoc>, ApiError> {
    let mut project = state.store.get_project(&id).await?; // FIX: war state.couch
    project.users.retain(|u| u.id != user_id);
    // Den Nutzer auch aus allen Aufgaben-Zuweisungen entfernen.
    for task in &mut project.tasks {
        task.assignee_ids.retain(|uid| uid != &user_id);
    }
    let updated = state.store.put_project(project).await?; // FIX: war state.couch
    publish_update(&state, &id).await;
    Ok(Json(updated))
}

// ------------------------------------------------------------------
// SSE-Events
// ------------------------------------------------------------------

/// GET /api/projects/:id/events – Server-Sent Events Stream für ein Projekt.
///
/// FIX: Bei `RecvError::Closed` (Sender gedroppt) gab der alte Code einen
/// Heartbeat zurück und lief endlos weiter. Jetzt wird der Stream korrekt
/// mit `None` beendet, wenn der Kanal geschlossen ist.
async fn project_events(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    let mut events = state.events.lock().await;
    let tx = events
        .entry(id.clone())
        .or_insert_with(|| broadcast::channel::<String>(100).0)
        .clone();
    let rx = tx.subscribe();
    drop(events);

    let out = stream::unfold(rx, move |mut rx| async move {
        match rx.recv().await {
            // Normale Update-Nachricht weiterleiten.
            Ok(msg) => Some((Ok(Event::default().event("project_update").data(msg)), rx)),
            // RecvError::Lagged: Nachrichten übersprungen → Heartbeat senden, weiterlaufen.
            Err(broadcast::error::RecvError::Lagged(_)) => {
                Some((Ok(Event::default().event("heartbeat").data("ping")), rx))
            }
            // FIX: RecvError::Closed: Sender ist weg → Stream beenden statt Endlosschleife.
            Err(broadcast::error::RecvError::Closed) => None,
        }
    });
    Sse::new(out)
}

// ------------------------------------------------------------------
// MCP-Endpunkte
// ------------------------------------------------------------------

/// GET /mcp/tools – Verfügbare Tools auflisten.
async fn list_tools() -> Json<Vec<ToolDef>> {
    Json(vec![
        ToolDef {
            name: "list_projects",
            description: "List all projects",
        },
        ToolDef {
            name: "create_project",
            description: "Create project",
        },
        ToolDef {
            name: "get_project",
            description: "Get one project",
        },
        ToolDef {
            name: "create_task",
            description: "Create a task",
        },
        ToolDef {
            name: "update_task",
            description: "Update a task",
        },
        ToolDef {
            name: "move_task",
            description: "Move a task between columns",
        },
        ToolDef {
            name: "delete_task",
            description: "Delete a task",
        },
        ToolDef {
            name: "summarize_board",
            description: "Summarize board status",
        },
    ])
}

/// POST /mcp/call – Ein Tool aufrufen.
async fn call_tool(
    State(state): State<AppState>,
    Json(call): Json<McpCall>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let out = match call.tool.as_str() {
        "list_projects" => serde_json::to_value(state.store.list_projects().await?)?,
        "get_project" => {
            let id = call.arguments["id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("id missing".into()))?;
            serde_json::to_value(state.store.get_project(id).await?)?
        }
        "create_project" => {
            let title = call.arguments["title"]
                .as_str()
                .unwrap_or("Untitled Project");
            let project = default_project(title.to_string());
            serde_json::to_value(state.store.create_project(project).await?)?
        }
        "create_task" => {
            let project_id = call.arguments["project_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("project_id missing".into()))?;
            let mut project = state.store.get_project(project_id).await?;
            let task = Task {
                id: Uuid::new_v4().to_string(),
                title: call.arguments["title"]
                    .as_str()
                    .unwrap_or("New task")
                    .to_string(),
                description: call.arguments["description"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                column_id: call.arguments["column_id"]
                    .as_str()
                    .unwrap_or(project.columns.first().map(|c| c.id.as_str()).unwrap_or(""))
                    .to_string(),
                assignee_ids: vec![],
                labels: vec![],
                order: project.tasks.len() as i32,
                created_at: Utc::now().to_rfc3339(),
                updated_at: Utc::now().to_rfc3339(),
            };
            project.tasks.push(task);
            serde_json::to_value(state.store.put_project(project).await?)?
        }
        "update_task" => {
            let project_id = call.arguments["project_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("project_id missing".into()))?;
            let task_id = call.arguments["task_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("task_id missing".into()))?;
            let mut project = state.store.get_project(project_id).await?;
            if let Some(task) = project.tasks.iter_mut().find(|t| t.id == task_id) {
                if let Some(title) = call.arguments["title"].as_str() {
                    task.title = title.to_string();
                }
                if let Some(desc) = call.arguments["description"].as_str() {
                    task.description = desc.to_string();
                }
                task.updated_at = Utc::now().to_rfc3339();
            }
            serde_json::to_value(state.store.put_project(project).await?)?
        }
        "move_task" => {
            let project_id = call.arguments["project_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("project_id missing".into()))?;
            let task_id = call.arguments["task_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("task_id missing".into()))?;
            let column_id = call.arguments["column_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("column_id missing".into()))?;
            let mut project = state.store.get_project(project_id).await?;
            if let Some(task) = project.tasks.iter_mut().find(|t| t.id == task_id) {
                task.column_id = column_id.to_string();
                task.updated_at = Utc::now().to_rfc3339();
            }
            serde_json::to_value(state.store.put_project(project).await?)?
        }
        "delete_task" => {
            let project_id = call.arguments["project_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("project_id missing".into()))?;
            let task_id = call.arguments["task_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("task_id missing".into()))?;
            let mut project = state.store.get_project(project_id).await?;
            project.tasks.retain(|t| t.id != task_id);
            serde_json::to_value(state.store.put_project(project).await?)?
        }
        "summarize_board" => {
            let project_id = call.arguments["project_id"]
                .as_str()
                .ok_or_else(|| ApiError::BadRequest("project_id missing".into()))?;
            let project = state.store.get_project(project_id).await?;
            let summary: Vec<_> = project
                .columns
                .iter()
                .map(|c| {
                    let count = project.tasks.iter().filter(|t| t.column_id == c.id).count();
                    format!("{}: {} tasks", c.title, count)
                })
                .collect();
            serde_json::json!({"project": project.title, "summary": summary.join(", ")})
        }
        _ => return Err(ApiError::BadRequest("unknown tool".into())),
    };
    Ok(Json(out))
}

// ------------------------------------------------------------------
// Hilfsfunktionen
// ------------------------------------------------------------------

/// Sendet eine SSE-Benachrichtigung an alle aktiven Listener eines Projekts.
async fn publish_update(state: &AppState, project_id: &str) {
    let events = state.events.lock().await;
    if let Some(tx) = events.get(project_id) {
        let _ = tx.send(project_id.to_string());
    }
}

// ------------------------------------------------------------------
// CouchDB-Implementierung
// ------------------------------------------------------------------

impl CouchDb {
    /// Stellt sicher, dass die Datenbank existiert (idempotenter PUT).
    async fn ensure_db(&self) -> anyhow::Result<()> {
        let url = format!("{}/{}", self.base_url, self.db);
        let resp = self.client.put(url).send().await?;
        // 412 Precondition Failed bedeutet: DB existiert bereits – kein Fehler.
        if !(resp.status().is_success() || resp.status().as_u16() == 412) {
            anyhow::bail!("Failed to ensure DB");
        }
        Ok(())
    }

    /// Listet alle Dokumente in der Datenbank auf.
    async fn list_projects(&self) -> Result<Vec<ProjectDoc>, ApiError> {
        #[derive(Deserialize)]
        struct AllDocs {
            rows: Vec<Row>,
        }
        #[derive(Deserialize)]
        struct Row {
            doc: Option<ProjectDoc>,
        }

        let url = format!("{}/{}/_all_docs?include_docs=true", self.base_url, self.db);
        let rows: AllDocs = self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(rows.rows.into_iter().filter_map(|r| r.doc).collect())
    }

    /// Legt ein neues Dokument in CouchDB an (POST).
    async fn create_project(&self, mut project: ProjectDoc) -> Result<ProjectDoc, ApiError> {
        if project.id.is_empty() {
            project.id = Uuid::new_v4().to_string();
        }
        let url = format!("{}/{}", self.base_url, self.db);
        let res: serde_json::Value = self
            .client
            .post(url)
            .json(&project)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        project.rev = res["rev"].as_str().map(ToString::to_string);
        Ok(project)
    }

    /// Liest ein einzelnes Dokument aus CouchDB.
    async fn get_project(&self, id: &str) -> Result<ProjectDoc, ApiError> {
        let url = format!("{}/{}/{}", self.base_url, self.db, id);
        let proj = self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json::<ProjectDoc>()
            .await?;
        Ok(proj)
    }

    /// Schreibt ein vorhandenes Dokument zurück (PUT mit Rev).
    async fn put_project(&self, mut project: ProjectDoc) -> Result<ProjectDoc, ApiError> {
        let id = project.id.clone();
        let url = format!("{}/{}/{}", self.base_url, self.db, id);
        let res: serde_json::Value = self
            .client
            .put(url)
            .json(&project)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        project.rev = res["rev"].as_str().map(ToString::to_string);
        Ok(project)
    }

    /// Löscht ein Dokument in CouchDB (erfordert aktuelle Rev).
    async fn delete_project(&self, id: &str, rev: &str) -> Result<(), ApiError> {
        let url = format!("{}/{}/{}?rev={}", self.base_url, self.db, id, rev);
        self.client.delete(url).send().await?.error_for_status()?;
        Ok(())
    }
}

// ------------------------------------------------------------------
// DataStore-Delegation
// ------------------------------------------------------------------

impl DataStore {
    async fn list_projects(&self) -> Result<Vec<ProjectDoc>, ApiError> {
        match self {
            DataStore::Couch(c) => c.list_projects().await,
            DataStore::File(f) => f.list_projects().await,
        }
    }

    async fn create_project(&self, project: ProjectDoc) -> Result<ProjectDoc, ApiError> {
        match self {
            DataStore::Couch(c) => c.create_project(project).await,
            DataStore::File(f) => f.create_project(project).await,
        }
    }

    async fn get_project(&self, id: &str) -> Result<ProjectDoc, ApiError> {
        match self {
            DataStore::Couch(c) => c.get_project(id).await,
            DataStore::File(f) => f.get_project(id).await,
        }
    }

    async fn put_project(&self, project: ProjectDoc) -> Result<ProjectDoc, ApiError> {
        match self {
            DataStore::Couch(c) => c.put_project(project).await,
            DataStore::File(f) => f.put_project(project).await,
        }
    }

    async fn delete_project(&self, id: &str, rev: &str) -> Result<(), ApiError> {
        match self {
            DataStore::Couch(c) => c.delete_project(id, rev).await,
            DataStore::File(f) => f.delete_project(id, rev).await,
        }
    }
}

// ------------------------------------------------------------------
// FileStore-Implementierung
// ------------------------------------------------------------------

impl FileStore {
    /// Erstellt das Root-Verzeichnis, falls es nicht existiert.
    async fn ensure_db(&self) -> Result<(), ApiError> {
        tokio::fs::create_dir_all(&self.root).await?;
        Ok(())
    }

    /// Gibt den Dateipfad für ein Projekt zurück.
    fn project_path(&self, id: &str) -> PathBuf {
        self.root.join(format!("{id}.json"))
    }

    /// Liest alle JSON-Dateien im Root-Verzeichnis ein.
    async fn list_projects(&self) -> Result<Vec<ProjectDoc>, ApiError> {
        let mut out = vec![];
        let mut entries = tokio::fs::read_dir(&self.root).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let content = tokio::fs::read_to_string(path).await?;
            let project: ProjectDoc = serde_json::from_str(&content)?;
            out.push(project);
        }
        Ok(out)
    }

    /// Schreibt ein neues Projekt als JSON-Datei. Startet mit Rev "1".
    async fn create_project(&self, mut project: ProjectDoc) -> Result<ProjectDoc, ApiError> {
        if project.id.is_empty() {
            project.id = Uuid::new_v4().to_string();
        }
        project.rev = Some("1".into());
        let content = serde_json::to_string_pretty(&project)?;
        tokio::fs::write(self.project_path(&project.id), content).await?;
        Ok(project)
    }

    /// Liest ein Projekt aus einer JSON-Datei.
    async fn get_project(&self, id: &str) -> Result<ProjectDoc, ApiError> {
        let path = self.project_path(id);
        if !path.exists() {
            return Err(ApiError::NotFound(format!("Project {id} not found")));
        }
        let content = tokio::fs::read_to_string(path).await?;
        Ok(serde_json::from_str(&content)?)
    }

    /// Überschreibt eine Projektdatei. Prüft Revisions-Übereinstimmung (optimistisches Locking).
    async fn put_project(&self, mut project: ProjectDoc) -> Result<ProjectDoc, ApiError> {
        let current = self.get_project(&project.id).await?;
        let current_rev = current.rev.unwrap_or_else(|| "0".into());
        let given_rev = project.rev.clone().unwrap_or_else(|| "".into());
        if given_rev != current_rev {
            return Err(ApiError::Conflict(format!(
                "Revision conflict: expected {current_rev}, got {given_rev}"
            )));
        }
        // Rev inkrementieren.
        let next_rev = current_rev.parse::<u64>().unwrap_or(0) + 1;
        project.rev = Some(next_rev.to_string());
        let content = serde_json::to_string_pretty(&project)?;
        tokio::fs::write(self.project_path(&project.id), content).await?;
        Ok(project)
    }

    /// Löscht eine Projektdatei nach Rev-Prüfung.
    async fn delete_project(&self, id: &str, rev: &str) -> Result<(), ApiError> {
        let current = self.get_project(id).await?;
        if current.rev.as_deref().unwrap_or("") != rev {
            return Err(ApiError::Conflict("Revision conflict on delete".into()));
        }
        tokio::fs::remove_file(self.project_path(id)).await?;
        Ok(())
    }
}

// ------------------------------------------------------------------
// Hilfsfunktion: Standard-Projekt
// ------------------------------------------------------------------

/// Erstellt ein Projekt mit drei Default-Spalten (Todo, In Progress, Done).
fn default_project(title: String) -> ProjectDoc {
    ProjectDoc {
        id: Uuid::new_v4().to_string(),
        rev: None,
        title,
        columns: vec![
            Column {
                id: Uuid::new_v4().to_string(),
                title: "Todo".into(),
                order: 0,
                color: "#90CAF9".into(),
            },
            Column {
                id: Uuid::new_v4().to_string(),
                title: "In Progress".into(),
                order: 1,
                color: "#FFCC80".into(),
            },
            Column {
                id: Uuid::new_v4().to_string(),
                title: "Done".into(),
                order: 2,
                color: "#A5D6A7".into(),
            },
        ],
        users: vec![],
        tasks: vec![],
    }
}

// ------------------------------------------------------------------
// Fehlerbehandlung
// ------------------------------------------------------------------

/// Einheitlicher Fehlertyp für alle API-Handler.
#[derive(Debug, thiserror::Error)]
enum ApiError {
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Conflict: {0}")]
    Conflict(String),
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// Konvertiert `ApiError` in eine HTTP-Antwort mit JSON-Body `{"error": "..."}`.
impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, msg) = match self {
            ApiError::NotFound(m) => (StatusCode::NOT_FOUND, m),
            ApiError::BadRequest(m) => (StatusCode::BAD_REQUEST, m),
            ApiError::Conflict(m) => (StatusCode::CONFLICT, m),
            ApiError::Reqwest(e) => (StatusCode::BAD_GATEWAY, e.to_string()),
            ApiError::Io(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            ApiError::Json(e) => (StatusCode::BAD_REQUEST, e.to_string()),
        };
        (status, Json(serde_json::json!({"error": msg}))).into_response()
    }
}