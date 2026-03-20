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
    routing::{delete, get, post, put},
    Router,
};
use reqwest::Client;
use tokio::sync::Mutex;
use tower_http::{cors::CorsLayer, services::ServeDir};
use tracing::info;

use axum::response::IntoResponse;
use config::Config;
use controllers::*;
use middleware::{auth_guard, print_startup_banner, request_logger};
use services::*;
use state::AppState;
use store::*;

/// SPA-Fallback: Liefert index.html für client-seitiges Routing (/p/*).
async fn spa_fallback() -> impl IntoResponse {
    let html = tokio::fs::read_to_string("static/index.html")
        .await
        .unwrap_or_else(|_| "<!DOCTYPE html><html><body>Not found</body></html>".into());
    axum::response::Html(html)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // .env laden (falls vorhanden)
    let _ = dotenvy::dotenv();

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
        cli_sessions: Arc::new(Mutex::new(HashMap::new())),
        mcp_sessions: Arc::new(Mutex::new(HashMap::new())),
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

    // Background-Task: Abgelaufene CLI-Sessions aufräumen.
    {
        let cli_sessions = state.cli_sessions.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                let mut sessions = cli_sessions.lock().await;
                let cutoff = chrono::Utc::now() - chrono::Duration::minutes(5);
                sessions.retain(|_, s| s.created_at > cutoff);
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
        // CLI Device Auth
        .route("/auth/cli-init", post(cli_init))
        .route("/auth/cli-poll/:session_id", get(cli_poll))
        .route("/auth/cli-approve", post(cli_approve))
        // CLI Script & Installer
        .route("/install", get(serve_installer))
        .route("/cli/plankton", get(serve_cli_script))
        .route("/cli-login", get(cli_login_page))
        // Healthcheck (kein Auth)
        .route("/healthz", get(|| async { axum::Json(serde_json::json!({"status":"ok"})) }))
        // Öffentliche User-Liste
        .route("/api/users", get(public_list_users))
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
        // Git-Integration deaktiviert (siehe git_controller.rs)
        // .route("/api/projects/:id/git", get(get_git_config).put(update_git_config))
        // .route("/api/projects/:id/git/sync", post(git_sync))
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
        // MCP (Legacy + Streamable HTTP Transport)
        .route("/mcp/tools", get(list_tools))
        .route("/mcp/call", post(call_tool))
        .route("/mcp", post(mcp_jsonrpc).get(mcp_sse_stream).delete(mcp_session_delete))
        // Docs & Skill
        .route("/docs", get(docs_page))
        .route("/skill.md", get(skill_md))
        // SPA-Fallback: /p/* und /import liefert index.html (URL-Routing im Frontend).
        .route("/p/*rest", get(spa_fallback))
        .route("/import", get(spa_fallback))
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
