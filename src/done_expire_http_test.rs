//! HTTP-Integrationstests für `doneExpire` + `archiveDelete` am Projekt-Modell.
//!
//! Testet auf HTTP/Router-Ebene (Tower oneshot):
//! 1. `GET /api/projects` → alle Projekte enthalten `doneExpire` und `archiveDelete`
//! 2. Projekt ohne Felder → Response enthält `doneExpire: 10`, `archiveDelete: 90`
//! 3. MCP `update_project` mit `done_expire: -1` → gespeichert und abrufbar
//! 4. MCP `update_project` mit `archive_delete: 30` → gespeichert und abrufbar

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use axum::routing::{get, post};
    use axum::Router;
    use tower::ServiceExt;
    use tower_http::cors::CorsLayer;

    use crate::controllers::mcp_controller::call_tool;
    use crate::controllers::project_controller::list_projects;
    use crate::models::{Column, ProjectDoc};
    use crate::state::AppState;
    use crate::store::{DataStore, FileStore};

    // ─── Hilfsfunktionen ────────────────────────────────────────────────────

    async fn make_test_state() -> (AppState, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = DataStore::File(FileStore {
            root: dir.path().to_path_buf(),
        });
        store.ensure_users_dir().await.ok();

        let state = AppState {
            store,
            events: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            jwt_secret: "test-secret".into(),
            cli_sessions: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            mcp_sessions: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            oauth_clients: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            oauth_codes: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            oauth_refresh_tokens: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            write_locks: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            http_client: reqwest::Client::new(),
        };
        (state, dir)
    }

    async fn create_test_token(state: &AppState, name: &str, role: &str) -> String {
        use crate::models::auth::{hash_token_secret, AgentToken, TokenScope};
        use uuid::Uuid;
        let token_value = format!("plk_{}", Uuid::new_v4().simple());
        let token = AgentToken {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            token_hash: hash_token_secret(&token_value),
            role: role.to_string(),
            active: true,
            created_at: chrono::Utc::now().to_rfc3339(),
            description: String::new(),
            creator: String::new(),
            last_used: None,
            scope: TokenScope::Global,
            expires_at: None,
        };
        state.store.create_token(token).await.unwrap();
        token_value
    }

    /// Erstellt ein Testprojekt ohne done_expire/archive_delete (Legacy-Simulation).
    fn make_test_project_without_expire_fields(title: &str) -> ProjectDoc {
        use uuid::Uuid;
        let todo_id = Uuid::new_v4().to_string();
        ProjectDoc {
            id: Uuid::new_v4().to_string(),
            rev: None,
            title: title.to_string(),
            slug: crate::models::project::project_slugify(title),
            owner: None,
            webhook_url: None,
            columns: vec![Column {
                id: todo_id,
                title: "Todo".to_string(),
                slug: "TODO".to_string(),
                order: 0,
                color: "#90CAF9".to_string(),
                hidden: false,
                locked: true,
            }],
            users: vec![],
            tasks: vec![],
            git: None,
            order: 0,
            r#type: None,
            done_expire: None,    // Kein explizites Feld – Default-Verhalten
            archive_delete: None, // Kein explizites Feld – Default-Verhalten
        }
    }

    fn build_list_router(state: AppState) -> Router {
        Router::new()
            .route("/api/projects", get(list_projects))
            .layer(CorsLayer::permissive())
            .with_state(state)
    }

    fn build_mcp_router(state: AppState) -> Router {
        Router::new()
            .route("/mcp/call", post(call_tool))
            .layer(CorsLayer::permissive())
            .with_state(state)
    }

    // ─── Test 1: GET /api/projects → alle Projekte enthalten doneExpire + archiveDelete ─

    /// GET /api/projects muss für jedes Projekt `doneExpire` und `archiveDelete` enthalten.
    #[tokio::test]
    async fn test_list_projects_includes_done_expire_and_archive_delete() {
        let (state, _dir) = make_test_state().await;
        let token = create_test_token(&state, "tester", "admin").await;

        let project = make_test_project_without_expire_fields("ExpireFieldsListTest");
        state.store.create_project(project).await.unwrap();

        let app = build_list_router(state);

        let req = Request::builder()
            .method(Method::GET)
            .uri("/api/projects")
            .header("authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "GET /api/projects muss 200 liefern"
        );

        let body_bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        let projects = json.as_array().expect("Response muss ein Array sein");
        assert!(
            !projects.is_empty(),
            "Mind. ein Projekt muss vorhanden sein"
        );

        for project in projects {
            assert!(
                project.get("doneExpire").is_some(),
                "Jedes Projekt muss 'doneExpire' in der JSON-Response haben"
            );
            assert!(
                project.get("archiveDelete").is_some(),
                "Jedes Projekt muss 'archiveDelete' in der JSON-Response haben"
            );
        }
    }

    // ─── Test 2: Projekt ohne Felder → Response enthält doneExpire: 10, archiveDelete: 90 ─

    /// Ein Projekt ohne done_expire/archive_delete muss in der GET-Response die Defaults liefern.
    #[tokio::test]
    async fn test_project_without_expire_fields_returns_defaults_in_response() {
        let (state, _dir) = make_test_state().await;
        let token = create_test_token(&state, "tester", "admin").await;

        let project = make_test_project_without_expire_fields("ExpireDefaultHttpTest");
        let project_id = project.id.clone();
        state.store.create_project(project).await.unwrap();

        let app = build_list_router(state);

        let req = Request::builder()
            .method(Method::GET)
            .uri("/api/projects")
            .header("authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        let projects = json.as_array().unwrap();
        let our_project = projects
            .iter()
            .find(|p| p["_id"].as_str() == Some(&project_id))
            .expect("Testprojekt muss in der Liste vorhanden sein");

        assert_eq!(
            our_project["doneExpire"].as_i64(),
            Some(10),
            "Projekt ohne done_expire muss doneExpire=10 in der Response liefern"
        );
        assert_eq!(
            our_project["archiveDelete"].as_i64(),
            Some(90),
            "Projekt ohne archive_delete muss archiveDelete=90 in der Response liefern"
        );
    }

    // ─── Test 3: MCP update_project mit done_expire: -1 → gespeichert und abrufbar ─

    /// MCP update_project mit done_expire=-1 muss den Wert persistieren und in GET liefern.
    #[tokio::test]
    async fn test_mcp_update_done_expire_minus_one_persisted() {
        let (state, _dir) = make_test_state().await;
        let token = create_test_token(&state, "manager", "admin").await;

        let project = make_test_project_without_expire_fields("DoneExpireMinusOneTest");
        let project_id = project.id.clone();
        state.store.create_project(project).await.unwrap();

        // Via MCP done_expire auf -1 setzen
        let mcp_app = build_mcp_router(state.clone());
        let mcp_body = serde_json::json!({
            "tool": "update_project",
            "arguments": {
                "project_id": project_id,
                "done_expire": -1
            }
        })
        .to_string();

        let req = Request::builder()
            .method(Method::POST)
            .uri("/mcp/call")
            .header("authorization", format!("Bearer {}", token))
            .header("content-type", "application/json")
            .body(Body::from(mcp_body))
            .unwrap();

        let resp = mcp_app.oneshot(req).await.unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "POST /mcp/call update_project muss 200 liefern"
        );

        let body_bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let result: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(
            result["doneExpire"].as_i64(),
            Some(-1),
            "MCP-Response muss doneExpire=-1 enthalten"
        );

        // Dauerhaft gespeichert: erneut laden
        let updated = state.store.get_project(&project_id).await.unwrap();
        assert_eq!(
            updated.done_expire(),
            -1,
            "Nach MCP update_project muss done_expire=-1 persistiert sein"
        );
        // archive_delete bleibt auf Default (90)
        assert_eq!(
            updated.archive_delete(),
            90,
            "archive_delete muss nach done_expire-Update unverändert (Default 90) sein"
        );
    }

    // ─── Test 4: MCP update_project mit archive_delete: 30 → gespeichert und abrufbar ─

    /// MCP update_project mit archive_delete=30 muss den Wert persistieren und in GET liefern.
    #[tokio::test]
    async fn test_mcp_update_archive_delete_persisted() {
        let (state, _dir) = make_test_state().await;
        let token = create_test_token(&state, "manager", "admin").await;

        let project = make_test_project_without_expire_fields("ArchiveDelete30Test");
        let project_id = project.id.clone();
        state.store.create_project(project).await.unwrap();

        // Via MCP archive_delete auf 30 setzen
        let mcp_app = build_mcp_router(state.clone());
        let mcp_body = serde_json::json!({
            "tool": "update_project",
            "arguments": {
                "project_id": project_id,
                "archive_delete": 30
            }
        })
        .to_string();

        let req = Request::builder()
            .method(Method::POST)
            .uri("/mcp/call")
            .header("authorization", format!("Bearer {}", token))
            .header("content-type", "application/json")
            .body(Body::from(mcp_body))
            .unwrap();

        let resp = mcp_app.oneshot(req).await.unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "POST /mcp/call update_project muss 200 liefern"
        );

        let body_bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let result: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(
            result["archiveDelete"].as_i64(),
            Some(30),
            "MCP-Response muss archiveDelete=30 enthalten"
        );

        // Dauerhaft gespeichert: erneut laden
        let updated = state.store.get_project(&project_id).await.unwrap();
        assert_eq!(
            updated.archive_delete(),
            30,
            "Nach MCP update_project muss archive_delete=30 persistiert sein"
        );
        // done_expire bleibt auf Default (10)
        assert_eq!(
            updated.done_expire(),
            10,
            "done_expire muss nach archive_delete-Update unverändert (Default 10) sein"
        );
    }
}
