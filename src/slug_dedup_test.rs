//! Integrationstests für Slug-Deduplizierung bei POST /api/projects.
//!
//! Feature: `create_project`-Handler
//! - Zwei Projekte mit gleichem Titel → zweites bekommt Slug mit `-2`-Suffix.
//! - Drei Projekte → drittes bekommt `-3`, etc.

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::post;
    use axum::Router;
    use tower::ServiceExt;

    use crate::controllers::project_controller::create_project;
    use crate::services::project_service::default_project;
    use crate::state::AppState;
    use crate::store::{DataStore, FileStore};

    async fn make_test_state() -> (AppState, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = DataStore::File(FileStore {
            root: dir.path().to_path_buf(),
        });
        store.ensure_users_dir().await.ok();

        let state = AppState {
            store,
            events: Arc::new(Mutex::new(HashMap::new())),
            jwt_secret: "test-secret".into(),
            cli_sessions: Arc::new(Mutex::new(HashMap::new())),
            mcp_sessions: Arc::new(Mutex::new(HashMap::new())),
            oauth_clients: Arc::new(Mutex::new(Vec::new())),
            oauth_codes: Arc::new(Mutex::new(HashMap::new())),
            oauth_refresh_tokens: Arc::new(Mutex::new(HashMap::new())),
            write_locks: Arc::new(Mutex::new(HashMap::new())),
            http_client: reqwest::Client::new(),
            last_maintenance_run: Arc::new(tokio::sync::RwLock::new(None)),
            started_at: chrono::Utc::now(),
            attachment_store: None,
        };
        (state, dir)
    }

    fn make_app(state: AppState) -> Router {
        Router::new()
            .route("/api/projects", post(create_project))
            .with_state(state)
    }

    async fn create_project_with_title(app: Router, title: &str) -> serde_json::Value {
        // Nutze default_project, setze aber den Slug leer, damit der Handler ihn generiert.
        let mut proj = default_project(title.to_string());
        proj.slug = String::new(); // Handler soll Slug generieren
        let body = serde_json::to_string(&proj).expect("serialize");
        let req = Request::builder()
            .method("POST")
            .uri("/api/projects")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();
        let resp = app.oneshot(req).await.expect("request");
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "create_project should return 200"
        );
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        serde_json::from_slice(&bytes).expect("valid JSON")
    }

    // -----------------------------------------------------------------------
    // Test 1: Zweites Projekt gleichen Namens bekommt -2-Suffix
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_duplicate_slug_gets_suffix() {
        let (state, _dir) = make_test_state().await;

        let app1 = make_app(state.clone());
        let p1 = create_project_with_title(app1, "Mein Projekt").await;
        let slug1 = p1["slug"].as_str().expect("slug");
        assert_eq!(slug1, "mein-projekt");

        let app2 = make_app(state.clone());
        let p2 = create_project_with_title(app2, "Mein Projekt").await;
        let slug2 = p2["slug"].as_str().expect("slug");
        assert_eq!(slug2, "mein-projekt-2");
    }

    // -----------------------------------------------------------------------
    // Test 2: Drittes Projekt mit gleichem Namen bekommt -3-Suffix
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_triple_duplicate_slug() {
        let (state, _dir) = make_test_state().await;

        let app1 = make_app(state.clone());
        let p1 = create_project_with_title(app1, "Test").await;
        assert_eq!(p1["slug"].as_str().unwrap(), "test");

        let app2 = make_app(state.clone());
        let p2 = create_project_with_title(app2, "Test").await;
        assert_eq!(p2["slug"].as_str().unwrap(), "test-2");

        let app3 = make_app(state.clone());
        let p3 = create_project_with_title(app3, "Test").await;
        assert_eq!(p3["slug"].as_str().unwrap(), "test-3");
    }
}
