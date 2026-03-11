// ============================================================
// Plankton – Kanban-Backend (Axum + CouchDB oder File-Store)
// ============================================================
// REST-API für ein Kanban-Board mit MCP-Unterstützung.
// ============================================================

mod models;
mod error;
mod store;
mod state;
mod config;
mod services;
mod controllers;
mod middleware;

use std::{collections::HashMap, net::SocketAddr, path::PathBuf, sync::Arc};

use axum::{
    routing::{get, post, put},
    Router,
};
use reqwest::Client;
use tokio::sync::Mutex;
use tower_http::{cors::CorsLayer, services::ServeDir};
use tracing::info;

use config::Config;
use controllers::*;
use middleware::{auth_guard, print_startup_banner, request_logger};
use services::*;
use state::AppState;
use store::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cfg = Config::from_env();

    // Backend-Auswahl: CouchDB wenn URI gesetzt, sonst File-Store.
    let store = if let Some(base_url) = cfg.couch_uri {
        let couch = CouchDb {
            client: Client::new(),
            base_url,
            db: cfg.db,
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

    // JWT-Secret aus Config oder zufällig generiert.
    let jwt_secret = cfg.jwt_secret.unwrap_or_else(|| {
        use rand::Rng;
        let secret: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(64)
            .map(char::from)
            .collect();
        info!("generated random JWT secret (set JWT_SECRET env var for persistence)");
        secret
    });

    let port = cfg.port;

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
        .route("/api/projects/:id/tasks/batch-move", post(batch_move_tasks))
        .route("/api/projects/:id/import", post(import_tasks))
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
        .route("/api/projects/:id/git", get(get_git_config).put(update_git_config))
        .route("/api/projects/:id/git/sync", post(git_sync))
        .route("/api/projects/:id/events", get(project_events))
        // Admin-Routen
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
        // Middleware
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
