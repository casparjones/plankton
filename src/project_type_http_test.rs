//! HTTP-Integrationstests für das `type`-Feld am Project-Modell.
//!
//! Testet auf HTTP/Router-Ebene (nicht Unit-Tests des internen Codes):
//! 1. `GET /api/projects` → Response enthält `"type"` Feld für alle Projekte
//! 2. Projekt ohne `type` → `"type": "kanban"` in Response
//! 3. `POST /mcp/call` mit `update_project` + `type: "list"` → gespeichert und abrufbar
//! 4. Ungültiger `type`-Wert → wird auf bestehenden Wert gefallbackt (kein Crash)

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

    fn make_test_project_without_type(title: &str) -> ProjectDoc {
        use uuid::Uuid;
        let todo_id = Uuid::new_v4().to_string();
        let _now = chrono::Utc::now().to_rfc3339();
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
            r#type: None, // Kein explizites type – Default-Verhalten
            done_expire: None,
            archive_delete: None,
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

    // ─── Test 1: GET /api/projects → alle Projekte haben `"type"` Feld ─────

    #[tokio::test]
    async fn test_list_projects_includes_type_field() {
        let (state, _dir) = make_test_state().await;
        let token = create_test_token(&state, "tester", "admin").await;

        let project = make_test_project_without_type("TypeFieldTest");
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
                project.get("type").is_some(),
                "Jedes Projekt muss ein 'type'-Feld in der JSON-Response haben"
            );
        }
    }

    // ─── Test 2: Projekt ohne `type` → Response enthält `"type": "kanban"` ─

    #[tokio::test]
    async fn test_project_without_type_returns_kanban_in_response() {
        let (state, _dir) = make_test_state().await;
        let token = create_test_token(&state, "tester", "admin").await;

        let project = make_test_project_without_type("KanbanDefaultHttpTest");
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
            .expect("Unser Testprojekt muss in der Liste vorhanden sein");

        assert_eq!(
            our_project["type"].as_str(),
            Some("kanban"),
            "Projekt ohne explizites type muss 'kanban' in der Response liefern"
        );
    }

    // ─── Test 3: update_project via MCP mit type="list" → gespeichert und abrufbar ─

    #[tokio::test]
    async fn test_mcp_update_project_type_list_persisted() {
        let (state, _dir) = make_test_state().await;
        let token = create_test_token(&state, "manager", "admin").await;

        let project = make_test_project_without_type("McpTypeListTest");
        let project_id = project.id.clone();
        state.store.create_project(project).await.unwrap();

        // Zunächst kanban (Default)
        let loaded = state.store.get_project(&project_id).await.unwrap();
        assert_eq!(loaded.project_type(), "kanban", "Vorher: kanban (Default)");

        // Via MCP type auf "list" setzen
        let mcp_app = build_mcp_router(state.clone());
        let mcp_body = serde_json::json!({
            "tool": "update_project",
            "arguments": {
                "project_id": project_id,
                "type": "list"
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
            result["type"].as_str(),
            Some("list"),
            "MCP-Response muss type='list' enthalten"
        );

        // Dauerhaft gespeichert: erneut laden
        let updated = state.store.get_project(&project_id).await.unwrap();
        assert_eq!(
            updated.project_type(),
            "list",
            "Nach MCP update_project muss type='list' persistiert sein"
        );
    }

    // ─── Test 4: Ungültiger type-Wert → Fallback auf bestehenden Typ (kein Crash) ─

    #[tokio::test]
    async fn test_mcp_update_project_invalid_type_fallback() {
        let (state, _dir) = make_test_state().await;
        let token = create_test_token(&state, "manager", "admin").await;

        let project = make_test_project_without_type("McpInvalidTypeTest");
        let project_id = project.id.clone();
        state.store.create_project(project).await.unwrap();

        // Ungültigen type-Wert senden
        let mcp_app = build_mcp_router(state.clone());
        let mcp_body = serde_json::json!({
            "tool": "update_project",
            "arguments": {
                "project_id": project_id,
                "type": "invalid-board-type"
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
        // Kein 500-Fehler erwartet – der Handler normalisiert ungültige Werte
        assert_ne!(
            resp.status(),
            StatusCode::INTERNAL_SERVER_ERROR,
            "Ungültiger type darf keinen 500-Fehler erzeugen"
        );

        let body_bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let result: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        // Der normalisierte Wert muss "kanban" sein (Fallback: project_type() = "kanban")
        let returned_type = result["type"].as_str().unwrap_or("");
        assert!(
            returned_type == "kanban" || returned_type == "list",
            "Ungültiger type muss auf einen bekannten Wert normalisiert werden, bekommen: '{returned_type}'"
        );

        // Im Store gespeichert: muss ebenfalls einen gültigen Wert haben
        let updated = state.store.get_project(&project_id).await.unwrap();
        let effective_type = updated.project_type();
        assert!(
            effective_type == "kanban" || effective_type == "list",
            "Im Store gespeicherter type muss gültig sein, bekommen: '{effective_type}'"
        );
    }
}
