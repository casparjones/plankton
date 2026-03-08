// ============================================================
// Plankton – Kanban-Backend (Axum + CouchDB oder File-Store)
// ============================================================
// Dieses Modul stellt eine REST-API für ein Kanban-Board bereit.
// Als Storage-Backend kann entweder CouchDB (via COUCHDB_URI)
// oder ein einfacher JSON-File-Store (./data/projects) verwendet werden.
// Zusätzlich gibt es einen minimalen MCP-Endpunkt (/mcp/*) für
// Tool-basierte KI-Zugriffe sowie SSE-Events pro Projekt.
// ============================================================

use std::{collections::HashMap, net::SocketAddr, path::PathBuf, sync::Arc, time::Instant};

use axum::{
    extract::{Path, Query, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{sse::Event, IntoResponse, Response, Sse},
    routing::{get, post, put},
    Json, Router,
};
use chrono::{Local, Utc};
use futures::{stream, Stream};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, Mutex};
use tower_http::{cors::CorsLayer, services::ServeDir};
use tracing::info;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header as JwtHeader, Validation};
use rand::rngs::OsRng;
use uuid::Uuid;

// ------------------------------------------------------------------
// App-State
// ------------------------------------------------------------------

/// Zentraler Anwendungs-State, der von Axum in alle Handler injiziert wird.
#[derive(Clone)]
struct AppState {
    store: DataStore,
    events: Arc<Mutex<HashMap<String, broadcast::Sender<String>>>>,
    jwt_secret: String,
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
    /// Versteckte Spalten (z.B. _archive) werden im Frontend nicht angezeigt.
    #[serde(default)]
    hidden: bool,
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
#[serde(default)]
struct Task {
    id: String,
    title: String,
    description: String,
    /// ID der Spalte, in der sich die Aufgabe befindet.
    column_id: String,
    /// ID der vorherigen Spalte (für Undo / Audit).
    previous_row: String,
    assignee_ids: Vec<String>,
    labels: Vec<String>,
    /// Reihenfolge innerhalb der Spalte.
    order: i32,
    /// Story Points (0-100).
    points: i32,
    /// Zugewiesener Bearbeiter.
    worker: String,
    /// Erstellt von.
    creator: String,
    /// Audit-Log: z.B. "2026-03-08 14:30 moved from Todo to In Progress".
    logs: Vec<String>,
    /// Kommentare: z.B. "Frank: Bitte Prio erhöhen".
    comments: Vec<String>,
    created_at: String,
    updated_at: String,
}

impl Default for Task {
    fn default() -> Self {
        Self {
            id: String::new(),
            title: String::new(),
            description: String::new(),
            column_id: String::new(),
            previous_row: String::new(),
            assignee_ids: vec![],
            labels: vec![],
            order: 0,
            points: 0,
            worker: String::new(),
            creator: String::new(),
            logs: vec![],
            comments: vec![],
            created_at: String::new(),
            updated_at: String::new(),
        }
    }
}

// ------------------------------------------------------------------
// Auth-Datenmodelle
// ------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
struct AuthUser {
    id: String,
    username: String,
    display_name: String,
    #[serde(default)]
    password_hash: String,
    role: String,
    created_at: String,
    updated_at: String,
    #[serde(default = "default_true")]
    active: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Claims {
    sub: String,
    username: String,
    display_name: String,
    role: String,
    exp: usize,
    #[serde(default)]
    must_change_password: bool,
}

#[derive(Debug, Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct ChangePasswordRequest {
    old_password: String,
    new_password: String,
}

#[derive(Debug, Deserialize)]
struct CreateAuthUserRequest {
    username: String,
    display_name: String,
    password: String,
    role: String,
}

#[derive(Debug, Deserialize)]
struct UpdateAuthUserRequest {
    display_name: Option<String>,
    role: Option<String>,
    active: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct ResetPasswordRequest {
    password: String,
}

// ------------------------------------------------------------------
// Agent-Token Datenmodell
// ------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
struct AgentToken {
    id: String,
    name: String,
    token: String,
    role: String,
    #[serde(default = "default_true")]
    active: bool,
    created_at: String,
}

#[derive(Debug, Deserialize)]
struct CreateTokenRequest {
    name: String,
    role: String,
}

#[derive(Debug, Deserialize)]
struct UpdateTokenRequest {
    name: Option<String>,
    role: Option<String>,
    active: Option<bool>,
}

fn generate_agent_token() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 24] = rng.gen();
    let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
    format!("plk_{}", hex)
}

// ------------------------------------------------------------------
// Request/Response-Hilfstypen
// ------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct DeleteQuery {
    rev: String,
}

/// Query-Parameter für GET /projects/:id – optionales Archiv-Flag.
#[derive(Debug, Deserialize)]
struct GetProjectQuery {
    #[serde(default)]
    include_archived: bool,
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
#[derive(Debug, Serialize, Clone)]
struct ToolDef {
    name: &'static str,
    description: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    roles: Option<&'static [&'static str]>,
}

/// JSON-RPC 2.0 Request
#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: Option<String>,
    method: String,
    #[serde(default)]
    params: serde_json::Value,
    id: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 Response
#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
    id: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
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

    // JWT-Secret aus Env-Variable oder zufällig generiert.
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| {
        use rand::Rng;
        let secret: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(64)
            .map(char::from)
            .collect();
        info!("generated random JWT secret (set JWT_SECRET env var for persistence)");
        secret
    });

    let state = AppState {
        store,
        events: Arc::new(Mutex::new(HashMap::new())),
        jwt_secret,
    };

    // Users-Verzeichnis sicherstellen und Default-Admin anlegen.
    state.store.ensure_users_dir().await?;
    ensure_default_admin(&state.store).await?;

    // Background-Task: Archivierung von Tasks die ≥14 Tage in "Done" liegen.
    {
        let archive_store = state.store.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(86400));
            loop {
                interval.tick().await;
                if let Err(e) = archive_old_tasks(&archive_store).await {
                    tracing::error!("Archivierungs-Fehler: {e}");
                }
            }
        });
    }

    // Router: Auth + REST-API + Admin + MCP + Statische Dateien.
    let app = Router::new()
        // Auth-Routen (öffentlich, kein Guard)
        .route("/auth/login", post(auth_login))
        .route("/auth/logout", post(auth_logout))
        .route("/auth/me", get(auth_me))
        .route("/auth/change-password", post(auth_change_password))
        // Projekt-API
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
        // Admin-Routen (Guard prüft admin-Rolle)
        .route(
            "/api/admin/users",
            get(admin_list_users).post(admin_create_user),
        )
        .route(
            "/api/admin/users/:user_id",
            put(admin_update_user).delete(admin_delete_user),
        )
        .route("/api/admin/users/:user_id/password", put(admin_reset_password))
        // Admin-Token-Routen
        .route(
            "/api/admin/tokens",
            get(admin_list_tokens).post(admin_create_token),
        )
        .route(
            "/api/admin/tokens/:token_id",
            put(admin_update_token).delete(admin_delete_token),
        )
        // MCP (Legacy + JSON-RPC 2.0)
        .route("/mcp/tools", get(list_tools))
        .route("/mcp/call", post(call_tool))
        .route("/mcp", post(mcp_jsonrpc))
        // Docs
        .route("/docs", get(docs_page))
        // Statische Dateien
        .nest_service(
            "/",
            ServeDir::new("static").append_index_html_on_directories(true),
        )
        // Middleware (Reihenfolge: innerste zuerst)
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth_guard,
        ))
        .layer(axum::middleware::from_fn(request_logger))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr: SocketAddr = format!("0.0.0.0:{port}").parse()?;
    print_startup_banner(&port);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

// ------------------------------------------------------------------
// Background-Task: Automatische Archivierung
// ------------------------------------------------------------------

/// Prüft alle Projekte und verschiebt Tasks, die ≥14 Tage in "Done" liegen,
/// in die versteckte "_archive"-Spalte.
async fn archive_old_tasks(store: &DataStore) -> Result<(), ApiError> {
    let projects = store.list_projects().await?;
    let cutoff = Utc::now() - chrono::Duration::days(14);

    for mut project in projects {
        // Done- und Archive-Spalte finden.
        let done_col_id = project.columns.iter()
            .find(|c| c.title == "Done")
            .map(|c| c.id.clone());
        let archive_col_id = project.columns.iter()
            .find(|c| c.title == "_archive")
            .map(|c| c.id.clone());

        let (done_id, archive_id) = match (done_col_id, archive_col_id) {
            (Some(d), Some(a)) => (d, a),
            _ => continue, // Projekt hat keine Done- oder Archive-Spalte.
        };

        let mut changed = false;
        for task in &mut project.tasks {
            if task.column_id != done_id {
                continue;
            }
            // updated_at parsen und prüfen ob älter als 14 Tage.
            let updated = chrono::DateTime::parse_from_rfc3339(&task.updated_at)
                .map(|dt| dt.with_timezone(&Utc));
            if let Ok(dt) = updated {
                if dt < cutoff {
                    task.previous_row = task.column_id.clone();
                    task.column_id = archive_id.clone();
                    task.updated_at = Utc::now().to_rfc3339();
                    task.logs.push(format!("{} auto-archived",
                        Local::now().format("%Y-%m-%d")));
                    changed = true;
                }
            }
        }

        if changed {
            store.put_project(project).await?;
            tracing::info!("Archivierung: Tasks in Projekt verschoben");
        }
    }
    Ok(())
}

// ------------------------------------------------------------------
// Request-Logging Middleware (farbig, mit Dauer)
// ------------------------------------------------------------------

/// ANSI-Farbcodes für Terminal-Ausgabe.
const RESET: &str = "\x1b[0m";
const GREEN: &str = "\x1b[32m";
const BLUE: &str = "\x1b[34m";
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";

/// Gibt die passende ANSI-Farbe für eine HTTP-Methode zurück.
fn method_color(method: &str) -> &'static str {
    match method {
        "GET" => GREEN,
        "POST" => BLUE,
        "PUT" => YELLOW,
        "DELETE" => RED,
        _ => RESET,
    }
}

/// Gibt die passende ANSI-Farbe für einen HTTP-Status-Code zurück.
fn status_color(status: u16) -> &'static str {
    match status {
        200..=299 => GREEN,
        400..=499 => YELLOW,
        500..=599 => RED,
        _ => RESET,
    }
}

/// Middleware: Loggt jeden Request mit Methode, Pfad, Status und Dauer.
async fn request_logger(req: Request, next: Next) -> Response {
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    let start = Instant::now();

    let response = next.run(req).await;

    let status = response.status().as_u16();
    let duration = start.elapsed();
    let ms = duration.as_secs_f64() * 1000.0;
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");

    println!(
        "{DIM}[{timestamp}]{RESET} {BOLD}{mc}{method:<7}{RESET} {path:<40} {sc}{status}{RESET}  {DIM}{ms:.1}ms{RESET}",
        mc = method_color(&method),
        sc = status_color(status),
    );

    response
}

/// Gibt das Startup-Banner und die Routen-Tabelle im Terminal aus.
fn print_startup_banner(port: &str) {
    // Alle registrierten API-Routen (statisch definiert, da Axum keine Introspection bietet).
    let routes: &[(&str, &str)] = &[
        ("POST",   "/auth/login"),
        ("POST",   "/auth/logout"),
        ("GET",    "/auth/me"),
        ("POST",   "/auth/change-password"),
        ("GET",    "/api/projects"),
        ("POST",   "/api/projects"),
        ("GET",    "/api/projects/:id"),
        ("PUT",    "/api/projects/:id"),
        ("DELETE", "/api/projects/:id"),
        ("POST",   "/api/projects/:id/tasks"),
        ("PUT",    "/api/projects/:id/tasks/:task_id"),
        ("DELETE", "/api/projects/:id/tasks/:task_id"),
        ("POST",   "/api/projects/:id/tasks/:task_id/move"),
        ("POST",   "/api/projects/:id/columns"),
        ("PUT",    "/api/projects/:id/columns/:column_id"),
        ("DELETE", "/api/projects/:id/columns/:column_id"),
        ("POST",   "/api/projects/:id/users"),
        ("PUT",    "/api/projects/:id/users/:user_id"),
        ("DELETE", "/api/projects/:id/users/:user_id"),
        ("GET",    "/api/projects/:id/events"),
        ("GET",    "/api/admin/users"),
        ("POST",   "/api/admin/users"),
        ("PUT",    "/api/admin/users/:id"),
        ("DELETE", "/api/admin/users/:id"),
        ("PUT",    "/api/admin/users/:id/password"),
        ("GET",    "/api/admin/tokens"),
        ("POST",   "/api/admin/tokens"),
        ("PUT",    "/api/admin/tokens/:id"),
        ("DELETE", "/api/admin/tokens/:id"),
        ("GET",    "/mcp/tools"),
        ("POST",   "/mcp/call"),
        ("POST",   "/mcp (JSON-RPC 2.0)"),
        ("GET",    "/docs"),
    ];

    println!();
    println!("  {BOLD}🪼 Plankton v0.1.0{RESET}");
    println!("  {DIM}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━{RESET}");
    for (method, path) in routes {
        let mc = method_color(method);
        println!("  {mc}{BOLD}{method:<7}{RESET} {path}");
    }
    println!("  {DIM}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━{RESET}");
    println!("  {GREEN}listening on 0.0.0.0:{port}{RESET}");
    println!();
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
/// Standardmäßig werden archivierte Tasks und versteckte Spalten ausgefiltert.
/// Mit `?include_archived=true` werden alle Daten zurückgegeben.
async fn get_project(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<GetProjectQuery>,
) -> Result<Json<ProjectDoc>, ApiError> {
    let mut project = state.store.get_project(&id).await?;
    if !query.include_archived {
        // IDs der versteckten Spalten sammeln.
        let hidden_col_ids: Vec<String> = project.columns.iter()
            .filter(|c| c.hidden)
            .map(|c| c.id.clone())
            .collect();
        // Tasks in versteckten Spalten ausfiltern.
        project.tasks.retain(|t| !hidden_col_ids.contains(&t.column_id));
        // Versteckte Spalten selbst ausfiltern.
        project.columns.retain(|c| !c.hidden);
    }
    Ok(Json(project))
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
    headers: axum::http::HeaderMap,
    Json(mut task): Json<Task>,
) -> Result<Json<ProjectDoc>, ApiError> {
    let mut project = state.store.get_project(&id).await?;
    if task.id.is_empty() {
        task.id = Uuid::new_v4().to_string();
    }
    let now = Utc::now().to_rfc3339();
    task.created_at = now.clone();
    task.updated_at = now;
    // Creator aus JWT Claims.
    let user_name = extract_token_from_headers(&headers)
        .and_then(|t| validate_jwt(&t, &state.jwt_secret).ok())
        .map(|c| c.display_name)
        .unwrap_or_else(|| "anonymous".to_string());
    if task.creator.is_empty() {
        task.creator = user_name;
    }
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
    headers: axum::http::HeaderMap,
    Json(req): Json<MoveTaskRequest>,
) -> Result<Json<ProjectDoc>, ApiError> {
    let mut project = state.store.get_project(&id).await?;
    // Display Name aus JWT Claims.
    let user_name = extract_token_from_headers(&headers)
        .and_then(|t| validate_jwt(&t, &state.jwt_secret).ok())
        .map(|c| c.display_name)
        .unwrap_or_else(|| "anonymous".to_string());
    // Spaltennamen für den Log-Eintrag ermitteln.
    let column_name = |col_id: &str| -> String {
        project.columns.iter()
            .find(|c| c.id == col_id)
            .map(|c| c.title.clone())
            .unwrap_or_else(|| col_id.to_string())
    };
    if let Some(task) = project.tasks.iter_mut().find(|t| t.id == task_id) {
        let old_col = task.column_id.clone();
        let old_name = column_name(&old_col);
        let new_name = column_name(&req.column_id);
        task.previous_row = old_col;
        task.column_id = req.column_id;
        task.order = req.order.unwrap_or(task.order);
        task.updated_at = Utc::now().to_rfc3339();
        // Audit-Log mit Display Name.
        let log = format!("[{}] {} moved from {} to {}",
            user_name, Local::now().format("%Y-%m-%d %H:%M"), old_name, new_name);
        task.logs.push(log);
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

/// Alle verfügbaren MCP-Tools mit optionaler Rollen-Einschränkung.
fn all_tools() -> Vec<ToolDef> {
    vec![
        // Basis-Tools (alle Rollen)
        ToolDef { name: "list_projects", description: "List all projects", roles: None },
        ToolDef { name: "get_project", description: "Get one project by id", roles: None },
        ToolDef { name: "summarize_board", description: "Summarize board column counts", roles: None },
        // Manager-Tools
        ToolDef { name: "create_project", description: "Create a new project", roles: Some(&["manager", "admin"]) },
        ToolDef { name: "list_epics", description: "List columns as epics with task counts", roles: Some(&["manager", "admin"]) },
        ToolDef { name: "create_task", description: "Create a task in a project", roles: Some(&["manager", "admin"]) },
        ToolDef { name: "assign_task", description: "Assign a worker to a task", roles: Some(&["manager", "admin"]) },
        // Developer-Tools
        ToolDef { name: "get_assigned_tasks", description: "Get tasks assigned to the caller", roles: Some(&["developer"]) },
        ToolDef { name: "update_task", description: "Update task title/description/labels", roles: Some(&["developer", "manager", "admin"]) },
        ToolDef { name: "add_log", description: "Append a log entry to a task", roles: Some(&["developer", "tester", "manager", "admin"]) },
        ToolDef { name: "submit_for_review", description: "Mark task as ready for review", roles: Some(&["developer"]) },
        // Tester-Tools
        ToolDef { name: "get_review_queue", description: "Get tasks waiting for review", roles: Some(&["tester"]) },
        ToolDef { name: "add_comment", description: "Add a comment to a task", roles: Some(&["tester", "developer", "manager", "admin"]) },
        ToolDef { name: "approve_task", description: "Approve and move task to Done", roles: Some(&["tester", "manager", "admin"]) },
        ToolDef { name: "reject_task", description: "Reject task and move back with comment", roles: Some(&["tester", "manager", "admin"]) },
        // Weitere Tools
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
async fn list_tools(
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
async fn call_tool(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(call): Json<McpCall>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let (caller, _) = resolve_caller(&headers, &state).await;
    let out = execute_tool(&state, &call.tool, &call.arguments, &caller).await?;
    Ok(Json(out))
}

/// POST /mcp – JSON-RPC 2.0 MCP-Endpunkt.
async fn mcp_jsonrpc(
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
            let updated = state.store.put_project(project).await?;
            publish_update(state, project_id).await;
            Ok(serde_json::to_value(updated)?)
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
        // --- Agenten-Workflow-Tools ---
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
            let updated = state.store.put_project(project).await?;
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
                // Zurück in vorherige Spalte.
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
async fn docs_page() -> axum::response::Html<String> {
    axum::response::Html(generate_docs_html())
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

// ------------------------------------------------------------------
// Auth-Hilfsfunktionen
// ------------------------------------------------------------------

fn hash_password(password: &str) -> Result<String, ApiError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| ApiError::BadRequest(format!("Hash error: {e}")))?;
    Ok(hash.to_string())
}

fn verify_password(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash)
        .ok()
        .map(|h| Argon2::default().verify_password(password.as_bytes(), &h).is_ok())
        .unwrap_or(false)
}

fn create_jwt(user: &AuthUser, secret: &str, must_change_pw: bool) -> Result<String, ApiError> {
    let exp = Utc::now() + chrono::Duration::hours(8);
    let claims = Claims {
        sub: user.id.clone(),
        username: user.username.clone(),
        display_name: user.display_name.clone(),
        role: user.role.clone(),
        exp: exp.timestamp() as usize,
        must_change_password: must_change_pw,
    };
    encode(
        &JwtHeader::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| ApiError::BadRequest(format!("JWT error: {e}")))
}

fn extract_token_from_headers(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|cookie_str| {
            cookie_str
                .split(';')
                .map(str::trim)
                .find_map(|c| c.strip_prefix("plankton_token="))
                .filter(|t| !t.is_empty())
                .map(String::from)
        })
        .or_else(|| {
            headers
                .get("authorization")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "))
                .map(String::from)
        })
}

fn validate_jwt(token: &str, secret: &str) -> Result<Claims, ApiError> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|_| ApiError::Unauthorized("Invalid token".into()))
}

/// Serialisiert einen AuthUser ohne password_hash für API-Responses.
fn user_to_json(user: &AuthUser) -> serde_json::Value {
    serde_json::json!({
        "id": user.id,
        "username": user.username,
        "display_name": user.display_name,
        "role": user.role,
        "active": user.active,
        "created_at": user.created_at,
        "updated_at": user.updated_at,
    })
}

// ------------------------------------------------------------------
// Auth-Guard Middleware
// ------------------------------------------------------------------

async fn auth_guard(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Response {
    let path = req.uri().path().to_string();

    // Öffentliche Pfade: /auth/*, /docs und statische Dateien.
    if path.starts_with("/auth/")
        || path == "/docs"
        || (!path.starts_with("/api/") && !path.starts_with("/mcp/"))
    {
        return next.run(req).await;
    }

    // 1) JWT-Token versuchen (Cookie oder Bearer).
    let jwt_token = extract_token_from_headers(req.headers());
    if let Some(ref t) = jwt_token {
        if let Ok(claims) = validate_jwt(t, &state.jwt_secret) {
            if path.starts_with("/api/admin/") && claims.role != "admin" {
                return (
                    StatusCode::FORBIDDEN,
                    Json(serde_json::json!({"error": "Admin required"})),
                )
                    .into_response();
            }
            req.extensions_mut().insert(claims);
            return next.run(req).await;
        }
    }

    // 2) Agent-Token versuchen (nur Bearer-Header).
    let bearer = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string());

    if let Some(bearer_val) = bearer {
        if let Ok(agent_token) = state.store.get_token_by_value(&bearer_val).await {
            // Agent-Tokens dürfen nicht auf Admin-Routen zugreifen.
            if path.starts_with("/api/admin/") {
                return (
                    StatusCode::FORBIDDEN,
                    Json(serde_json::json!({"error": "Admin required"})),
                )
                    .into_response();
            }
            // Claims-kompatibles Objekt für Agent-Token erstellen.
            let claims = Claims {
                sub: agent_token.id.clone(),
                username: agent_token.name.clone(),
                display_name: agent_token.name.clone(),
                role: agent_token.role.clone(),
                exp: usize::MAX,
                must_change_password: false,
            };
            req.extensions_mut().insert(claims);
            return next.run(req).await;
        }
    }

    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({"error": "Not authenticated"})),
    )
        .into_response()
}

// ------------------------------------------------------------------
// Auth-Endpunkte
// ------------------------------------------------------------------

async fn auth_login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Response, ApiError> {
    let user = state
        .store
        .get_user_by_username(&payload.username)
        .await
        .map_err(|_| ApiError::Unauthorized("Invalid credentials".into()))?;

    if !user.active {
        return Err(ApiError::Unauthorized("Account deactivated".into()));
    }

    if !verify_password(&payload.password, &user.password_hash) {
        return Err(ApiError::Unauthorized("Invalid credentials".into()));
    }

    let must_change = payload.password == "admin" && user.username == "admin";
    let token = create_jwt(&user, &state.jwt_secret, must_change)?;

    let cookie = format!(
        "plankton_token={}; HttpOnly; Path=/; Max-Age=28800; SameSite=Lax",
        token
    );

    let mut response = Json(serde_json::json!({
        "user_id": user.id,
        "display_name": user.display_name,
        "role": user.role,
        "must_change_password": must_change,
    }))
    .into_response();

    response
        .headers_mut()
        .insert("set-cookie", cookie.parse().unwrap());

    Ok(response)
}

async fn auth_logout() -> Response {
    let cookie = "plankton_token=; HttpOnly; Path=/; Max-Age=0; SameSite=Lax";
    let mut response = Json(serde_json::json!({"ok": true})).into_response();
    response
        .headers_mut()
        .insert("set-cookie", cookie.parse().unwrap());
    response
}

async fn auth_me(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let token = extract_token_from_headers(&headers)
        .ok_or(ApiError::Unauthorized("Not authenticated".into()))?;
    let claims = validate_jwt(&token, &state.jwt_secret)?;
    Ok(Json(serde_json::json!({
        "user_id": claims.sub,
        "username": claims.username,
        "display_name": claims.display_name,
        "role": claims.role,
        "must_change_password": claims.must_change_password,
    })))
}

async fn auth_change_password(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<ChangePasswordRequest>,
) -> Result<Response, ApiError> {
    let token = extract_token_from_headers(&headers)
        .ok_or(ApiError::Unauthorized("Not authenticated".into()))?;
    let claims = validate_jwt(&token, &state.jwt_secret)?;

    let mut user = state.store.get_user(&claims.sub).await?;

    if !verify_password(&payload.old_password, &user.password_hash) {
        return Err(ApiError::BadRequest("Old password incorrect".into()));
    }

    if payload.new_password.len() < 4 {
        return Err(ApiError::BadRequest(
            "Password must be at least 4 characters".into(),
        ));
    }

    user.password_hash = hash_password(&payload.new_password)?;
    user.updated_at = Utc::now().to_rfc3339();
    state.store.update_user(user.clone()).await?;

    // Neues JWT ohne must_change_password ausstellen.
    let new_token = create_jwt(&user, &state.jwt_secret, false)?;
    let cookie = format!(
        "plankton_token={}; HttpOnly; Path=/; Max-Age=28800; SameSite=Lax",
        new_token
    );

    let mut response = Json(serde_json::json!({"ok": true})).into_response();
    response
        .headers_mut()
        .insert("set-cookie", cookie.parse().unwrap());

    Ok(response)
}

// ------------------------------------------------------------------
// Admin-Endpunkte (Nutzerverwaltung)
// ------------------------------------------------------------------

async fn admin_list_users(
    State(state): State<AppState>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let users = state.store.list_users().await?;
    Ok(Json(users.iter().map(user_to_json).collect()))
}

async fn admin_create_user(
    State(state): State<AppState>,
    Json(payload): Json<CreateAuthUserRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Prüfen ob Username bereits existiert.
    if state
        .store
        .get_user_by_username(&payload.username.to_lowercase())
        .await
        .is_ok()
    {
        return Err(ApiError::Conflict(format!(
            "Username '{}' already exists",
            payload.username
        )));
    }

    let now = Utc::now().to_rfc3339();
    let user = AuthUser {
        id: Uuid::new_v4().to_string(),
        username: payload.username.to_lowercase(),
        display_name: payload.display_name,
        password_hash: hash_password(&payload.password)?,
        role: payload.role,
        created_at: now.clone(),
        updated_at: now,
        active: true,
    };

    let created = state.store.create_user(user).await?;
    Ok(Json(user_to_json(&created)))
}

async fn admin_update_user(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    Json(payload): Json<UpdateAuthUserRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut user = state.store.get_user(&user_id).await?;

    if let Some(name) = payload.display_name {
        user.display_name = name;
    }
    if let Some(role) = payload.role {
        user.role = role;
    }
    if let Some(active) = payload.active {
        user.active = active;
    }
    user.updated_at = Utc::now().to_rfc3339();

    let updated = state.store.update_user(user).await?;
    Ok(Json(user_to_json(&updated)))
}

async fn admin_delete_user(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    headers: axum::http::HeaderMap,
) -> Result<StatusCode, ApiError> {
    // Admin kann sich nicht selbst löschen.
    if let Some(token) = extract_token_from_headers(&headers) {
        if let Ok(claims) = validate_jwt(&token, &state.jwt_secret) {
            if claims.sub == user_id {
                return Err(ApiError::BadRequest(
                    "Cannot delete your own account".into(),
                ));
            }
        }
    }
    state.store.delete_user(&user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn admin_reset_password(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    Json(payload): Json<ResetPasswordRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut user = state.store.get_user(&user_id).await?;
    user.password_hash = hash_password(&payload.password)?;
    user.updated_at = Utc::now().to_rfc3339();
    state.store.update_user(user).await?;
    Ok(Json(serde_json::json!({"ok": true})))
}

// ------------------------------------------------------------------
// Admin-Endpunkte: Agent-Token-Verwaltung
// ------------------------------------------------------------------

async fn admin_list_tokens(
    State(state): State<AppState>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let tokens = state.store.list_tokens().await?;
    Ok(Json(
        tokens
            .iter()
            .map(|t| {
                serde_json::json!({
                    "id": t.id,
                    "name": t.name,
                    "role": t.role,
                    "active": t.active,
                    "created_at": t.created_at,
                })
            })
            .collect(),
    ))
}

async fn admin_create_token(
    State(state): State<AppState>,
    Json(payload): Json<CreateTokenRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let token = AgentToken {
        id: Uuid::new_v4().to_string(),
        name: payload.name,
        token: generate_agent_token(),
        role: payload.role,
        active: true,
        created_at: Utc::now().to_rfc3339(),
    };
    let created = state.store.create_token(token).await?;
    // Einmalig den Token-String zurückgeben!
    Ok(Json(serde_json::json!({
        "id": created.id,
        "name": created.name,
        "token": created.token,
        "role": created.role,
        "active": created.active,
        "created_at": created.created_at,
    })))
}

async fn admin_update_token(
    State(state): State<AppState>,
    Path(token_id): Path<String>,
    Json(payload): Json<UpdateTokenRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut token = state.store.get_token(&token_id).await?;
    if let Some(name) = payload.name {
        token.name = name;
    }
    if let Some(role) = payload.role {
        token.role = role;
    }
    if let Some(active) = payload.active {
        token.active = active;
    }
    let updated = state.store.update_token(token).await?;
    Ok(Json(serde_json::json!({
        "id": updated.id,
        "name": updated.name,
        "role": updated.role,
        "active": updated.active,
        "created_at": updated.created_at,
    })))
}

async fn admin_delete_token(
    State(state): State<AppState>,
    Path(token_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.store.delete_token(&token_id).await?;
    Ok(Json(serde_json::json!({"ok": true})))
}

// ------------------------------------------------------------------
// Bootstrap: Standard-Admin beim ersten Start
// ------------------------------------------------------------------

async fn ensure_default_admin(store: &DataStore) -> Result<(), ApiError> {
    let users = store.list_users().await?;
    let has_admin = users.iter().any(|u| u.role == "admin");

    if !has_admin {
        let now = Utc::now().to_rfc3339();
        let admin = AuthUser {
            id: Uuid::new_v4().to_string(),
            username: "admin".into(),
            display_name: "Administrator".into(),
            password_hash: hash_password("admin")?,
            role: "admin".into(),
            created_at: now.clone(),
            updated_at: now,
            active: true,
        };
        store.create_user(admin).await?;
        println!(
            "  {}{}Default admin created{} (username: admin, password: admin)",
            BOLD, YELLOW, RESET
        );
    }
    Ok(())
}

// ------------------------------------------------------------------
// Hilfsfunktionen
// ------------------------------------------------------------------

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

    // ---- User-Management (immer Dateisystem-basiert) ----

    fn users_root(&self) -> PathBuf {
        PathBuf::from("data/users")
    }

    fn user_path(&self, id: &str) -> PathBuf {
        self.users_root().join(format!("{id}.json"))
    }

    async fn ensure_users_dir(&self) -> Result<(), ApiError> {
        tokio::fs::create_dir_all(self.users_root()).await?;
        Ok(())
    }

    async fn list_users(&self) -> Result<Vec<AuthUser>, ApiError> {
        let dir = self.users_root();
        if !dir.exists() {
            return Ok(vec![]);
        }
        let mut out = vec![];
        let mut entries = tokio::fs::read_dir(dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let content = tokio::fs::read_to_string(path).await?;
            let user: AuthUser = serde_json::from_str(&content)?;
            out.push(user);
        }
        Ok(out)
    }

    async fn get_user(&self, id: &str) -> Result<AuthUser, ApiError> {
        let path = self.user_path(id);
        if !path.exists() {
            return Err(ApiError::NotFound(format!("User {id} not found")));
        }
        let content = tokio::fs::read_to_string(path).await?;
        Ok(serde_json::from_str(&content)?)
    }

    async fn get_user_by_username(&self, username: &str) -> Result<AuthUser, ApiError> {
        let users = self.list_users().await?;
        users
            .into_iter()
            .find(|u| u.username == username)
            .ok_or_else(|| ApiError::NotFound(format!("User '{username}' not found")))
    }

    async fn create_user(&self, mut user: AuthUser) -> Result<AuthUser, ApiError> {
        if user.id.is_empty() {
            user.id = Uuid::new_v4().to_string();
        }
        let content = serde_json::to_string_pretty(&user)?;
        tokio::fs::write(self.user_path(&user.id), content).await?;
        Ok(user)
    }

    async fn update_user(&self, user: AuthUser) -> Result<AuthUser, ApiError> {
        let path = self.user_path(&user.id);
        if !path.exists() {
            return Err(ApiError::NotFound(format!("User {} not found", user.id)));
        }
        let content = serde_json::to_string_pretty(&user)?;
        tokio::fs::write(path, content).await?;
        Ok(user)
    }

    async fn delete_user(&self, id: &str) -> Result<(), ApiError> {
        let path = self.user_path(id);
        if !path.exists() {
            return Err(ApiError::NotFound(format!("User {id} not found")));
        }
        tokio::fs::remove_file(path).await?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // Token-Verwaltung (immer File-basiert)
    // ------------------------------------------------------------------

    fn tokens_root(&self) -> PathBuf {
        PathBuf::from("data/tokens")
    }

    fn token_path(&self, id: &str) -> PathBuf {
        self.tokens_root().join(format!("{id}.json"))
    }

    async fn ensure_tokens_dir(&self) -> Result<(), ApiError> {
        tokio::fs::create_dir_all(self.tokens_root()).await?;
        Ok(())
    }

    async fn list_tokens(&self) -> Result<Vec<AgentToken>, ApiError> {
        self.ensure_tokens_dir().await?;
        let mut tokens = Vec::new();
        let mut dir = tokio::fs::read_dir(self.tokens_root()).await?;
        while let Some(entry) = dir.next_entry().await? {
            if entry.path().extension().map(|e| e == "json").unwrap_or(false) {
                let data = tokio::fs::read_to_string(entry.path()).await?;
                let token: AgentToken = serde_json::from_str(&data)?;
                tokens.push(token);
            }
        }
        Ok(tokens)
    }

    async fn get_token(&self, id: &str) -> Result<AgentToken, ApiError> {
        let data = tokio::fs::read_to_string(self.token_path(id))
            .await
            .map_err(|_| ApiError::NotFound("Token not found".into()))?;
        Ok(serde_json::from_str(&data)?)
    }

    async fn get_token_by_value(&self, token_value: &str) -> Result<AgentToken, ApiError> {
        let tokens = self.list_tokens().await?;
        tokens
            .into_iter()
            .find(|t| t.token == token_value && t.active)
            .ok_or_else(|| ApiError::NotFound("Token not found or inactive".into()))
    }

    async fn create_token(&self, token: AgentToken) -> Result<AgentToken, ApiError> {
        self.ensure_tokens_dir().await?;
        let data = serde_json::to_string_pretty(&token)?;
        tokio::fs::write(self.token_path(&token.id), data).await?;
        Ok(token)
    }

    async fn update_token(&self, token: AgentToken) -> Result<AgentToken, ApiError> {
        let data = serde_json::to_string_pretty(&token)?;
        tokio::fs::write(self.token_path(&token.id), data).await?;
        Ok(token)
    }

    async fn delete_token(&self, id: &str) -> Result<(), ApiError> {
        tokio::fs::remove_file(self.token_path(id))
            .await
            .map_err(|_| ApiError::NotFound("Token not found".into()))?;
        Ok(())
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

/// Erstellt ein Projekt mit drei Default-Spalten (Todo, In Progress, Done) + versteckte _archive-Spalte.
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
                hidden: false,
            },
            Column {
                id: Uuid::new_v4().to_string(),
                title: "In Progress".into(),
                order: 1,
                color: "#FFCC80".into(),
                hidden: false,
            },
            Column {
                id: Uuid::new_v4().to_string(),
                title: "Done".into(),
                order: 2,
                color: "#A5D6A7".into(),
                hidden: false,
            },
            Column {
                id: Uuid::new_v4().to_string(),
                title: "_archive".into(),
                order: 99,
                color: "#444".into(),
                hidden: true,
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
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Forbidden: {0}")]
    Forbidden(String),
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
            ApiError::Unauthorized(m) => (StatusCode::UNAUTHORIZED, m),
            ApiError::Forbidden(m) => (StatusCode::FORBIDDEN, m),
            ApiError::Reqwest(e) => (StatusCode::BAD_GATEWAY, e.to_string()),
            ApiError::Io(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            ApiError::Json(e) => (StatusCode::BAD_REQUEST, e.to_string()),
        };
        (status, Json(serde_json::json!({"error": msg}))).into_response()
    }
}