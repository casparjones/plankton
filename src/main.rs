// ============================================================
// Plankton – Kanban-Backend (Axum + CouchDB oder File-Store)
// ============================================================
// REST-API für ein Kanban-Board mit MCP-Unterstützung.
// ============================================================

#[cfg(test)]
mod attachment_test;
#[cfg(test)]
mod auto_archive_job_test;
#[cfg(test)]
mod blocking_test;
#[cfg(test)]
mod burndown_test;
#[cfg(test)]
mod cli_write_test;
#[cfg(test)]
mod column_entered_at_http_test;
#[cfg(test)]
mod column_entered_at_test;
mod config;
mod controllers;
#[cfg(test)]
mod done_expire_http_test;
#[cfg(test)]
mod done_expire_test;
mod error;
#[cfg(test)]
mod maintenance_job_integration_test;
#[cfg(test)]
mod mcp_compat_test;
mod middleware;
mod models;
#[cfg(test)]
mod move_task_to_project_http_test;
#[cfg(test)]
mod move_task_to_project_test;
#[cfg(test)]
mod optimistic_locking_test;
#[cfg(test)]
mod project_reorder_test;
#[cfg(test)]
mod project_type_http_test;
#[cfg(test)]
mod project_type_test;
mod services;
#[cfg(test)]
mod slug_dedup_test;
mod state;
#[cfg(test)]
mod stats_columns_test;
mod store;
#[cfg(test)]
mod task_templates_test;
#[cfg(test)]
mod velocity_test;
#[cfg(test)]
mod webhook_test;

use std::{collections::HashMap, net::SocketAddr, path::PathBuf, sync::Arc};

use axum::{
    routing::{get, post, put},
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

    // S3 Attachment-Store initialisieren (nur wenn S3_BUCKET konfiguriert).
    let attachment_store: Option<Arc<dyn services::AttachmentStore>> =
        cfg.s3.as_ref().map(|s3_cfg| {
            info!("S3 attachment storage enabled (bucket: {})", s3_cfg.bucket);
            let store = services::attachment_service::build_s3_store(s3_cfg);
            Arc::new(store) as Arc<dyn services::AttachmentStore>
        });
    if attachment_store.is_none() {
        info!("S3_BUCKET not set — file attachment feature disabled");
    }

    let state = AppState {
        store,
        events: Arc::new(Mutex::new(HashMap::new())),
        jwt_secret,
        cli_sessions: Arc::new(Mutex::new(HashMap::new())),
        mcp_sessions: Arc::new(Mutex::new(HashMap::new())),
        oauth_clients: Arc::new(Mutex::new(Vec::new())),
        oauth_codes: Arc::new(Mutex::new(HashMap::new())),
        oauth_refresh_tokens: Arc::new(Mutex::new(HashMap::new())),
        write_locks: Arc::new(Mutex::new(HashMap::new())),
        http_client: Client::new(),
        last_maintenance_run: Arc::new(tokio::sync::RwLock::new(None)),
        started_at: chrono::Utc::now(),
        attachment_store,
    };

    // Users-Verzeichnis sicherstellen und Default-Admin anlegen.
    state.store.ensure_users_dir().await?;
    ensure_default_admin(&state.store).await?;

    // Background-Task: Stündlicher Wartungs-Job (Auto-Archivierung + Auto-Delete).
    {
        let maintenance_store = state.store.clone();
        let last_run = state.last_maintenance_run.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
            loop {
                interval.tick().await;
                match crate::services::project_service::run_maintenance_job(&maintenance_store)
                    .await
                {
                    Ok(()) => {
                        let mut w = last_run.write().await;
                        *w = Some(chrono::Utc::now());
                    }
                    Err(e) => {
                        tracing::error!("Maintenance-Job Fehler: {e}");
                    }
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

    let app = build_router(state);

    let addr: SocketAddr = format!("0.0.0.0:{port}").parse()?;
    print_startup_banner(&port);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

/// Baut den Axum-Router. Auch von Tests verwendbar.
pub fn build_router(state: state::AppState) -> axum::Router {
    let mut router = Router::new()
        // Auth-Routen (öffentlich, kein Guard)
        .route("/auth/login", post(auth_login))
        .route("/auth/logout", post(auth_logout))
        .route("/auth/me", get(auth_me))
        .route("/auth/change-password", post(auth_change_password))
        // OAuth 2.0 (MCP spec: endpoints at authorization base URL root)
        .route("/authorize", get(oauth_authorize))
        .route("/token", post(oauth_token))
        .route("/register", post(oauth_register))
        .route(
            "/.well-known/oauth-authorization-server",
            get(oauth_metadata),
        )
        .route(
            "/.well-known/oauth-protected-resource",
            get(oauth_protected_resource),
        )
        // OAuth paths with /oauth/ prefix (what claude.ai expects)
        .route("/oauth/authorize", get(oauth_authorize))
        .route("/oauth/token", post(oauth_token))
        .route("/oauth/register", post(oauth_register))
        // CLI Device Auth
        .route("/auth/cli-init", post(cli_init))
        .route("/auth/cli-poll/:session_id", get(cli_poll))
        .route("/auth/cli-approve", post(cli_approve))
        // CLI Script & Installer
        .route("/install", get(serve_installer))
        .route("/cli/plankton", get(serve_cli_script))
        .route("/cli-login", get(cli_login_page))
        // Healthcheck (kein Auth)
        .route(
            "/healthz",
            get(|| async { axum::Json(serde_json::json!({"status":"ok"})) }),
        )
        // Öffentliche User-Liste
        .route("/api/users", get(public_list_users))
        // Projekt-API
        .route("/api/projects/reorder", post(reorder_projects))
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
        .route(
            "/api/projects/:id/tasks/:task_id/comment",
            post(add_comment),
        )
        .route("/api/projects/:id/tasks/:task_id/move", post(move_task))
        .route("/api/projects/:id/tasks/reorder", post(reorder_tasks))
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
        .route(
            "/api/projects/:id/stats/columns",
            get(project_stats_columns),
        )
        .route(
            "/api/projects/:id/stats/velocity",
            get(project_stats_velocity),
        )
        .route(
            "/api/projects/:id/stats/burndown",
            get(project_stats_burndown),
        )
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
        .route(
            "/api/admin/users/:user_id/password",
            put(admin_reset_password),
        )
        .route(
            "/api/admin/tokens",
            get(admin_list_tokens).post(admin_create_token),
        )
        .route(
            "/api/admin/tokens/:token_id",
            put(admin_update_token).delete(admin_delete_token),
        )
        .route("/api/admin/system-status", get(admin_system_status))
        .route(
            "/api/admin/oauth-clients",
            get(admin_list_oauth_clients).post(admin_create_oauth_client),
        )
        // MCP (Legacy + Streamable HTTP Transport)
        .route("/mcp/tools", get(list_tools))
        .route("/mcp/call", post(call_tool))
        .route(
            "/mcp",
            post(mcp_jsonrpc)
                .get(mcp_sse_stream)
                .delete(mcp_session_delete),
        )
        // Incoming Webhooks (extern → Plankton)
        .route(
            "/webhook/projects/:slug/tasks/:task_id/move",
            post(incoming_move_task),
        )
        // Docs & Skill
        .route("/docs", get(docs_page))
        .route("/skill.md", get(skill_md))
        // SPA-Fallback
        .route("/p/*rest", get(spa_fallback))
        .route("/import", get(spa_fallback))
        // Statische Dateien
        .nest_service(
            "/",
            ServeDir::new("static").append_index_html_on_directories(true),
        );

    // File-Attachment-Routen nur registrieren wenn S3 konfiguriert ist.
    if state.attachment_store.is_some() {
        router = router
            .route(
                "/api/projects/:id/tasks/:task_id/attachments",
                post(upload_attachment).get(list_attachments),
            )
            .route(
                "/api/projects/:id/tasks/:task_id/attachments/:attachment_id",
                get(download_attachment).delete(delete_attachment),
            );
    }

    router
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth_guard,
        ))
        .layer(axum::middleware::from_fn(request_logger))
        .layer(CorsLayer::permissive())
        .with_state(state)
}
