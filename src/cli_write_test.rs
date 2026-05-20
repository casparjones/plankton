/// Integrationstests für CLI v0.2.0 – Write-Operationen (Ticket 1844f35e)
///
/// Prüft:
/// 1. POST /api/projects/:id/tasks/:task_id/comment – Kommentar hinzufügen (Happy Path)
/// 2. POST /api/projects/:id/tasks/:task_id/comment – fehlender text → 400
/// 3. POST /api/projects/:id/tasks/:task_id/comment – ohne Auth → 401
/// 4. CLI-Script enthält alle neuen v0.2.0-Subcommands (move, done, comment, create)
/// 5. CLI-Script meldet VERSION 0.2.0
#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use axum::routing::post;
    use axum::Router;
    use tower::ServiceExt;
    use tower_http::cors::CorsLayer;

    use crate::controllers::cli_controller::build_cli_script;
    use crate::controllers::task_controller::add_comment;
    use crate::models::{Column, ProjectDoc, Task};
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

    fn make_test_project() -> ProjectDoc {
        use uuid::Uuid;
        let todo_id = Uuid::new_v4().to_string();
        let done_id = Uuid::new_v4().to_string();
        let task_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        ProjectDoc {
            id: Uuid::new_v4().to_string(),
            rev: None,
            title: "Comment Test Project".to_string(),
            slug: "comment-test-project".to_string(),
            owner: None,
            webhook_url: None,
            columns: vec![
                Column {
                    id: todo_id.clone(),
                    title: "Todo".to_string(),
                    slug: "TODO".to_string(),
                    order: 0,
                    color: "#90CAF9".to_string(),
                    hidden: false,
                    locked: true,
                },
                Column {
                    id: done_id,
                    title: "Done".to_string(),
                    slug: "DONE".to_string(),
                    order: 1,
                    color: "#A5D6A7".to_string(),
                    hidden: false,
                    locked: false,
                },
            ],
            users: vec![],
            tasks: vec![Task {
                id: task_id,
                slug: "my-task".to_string(),
                title: "My Task".to_string(),
                column_id: todo_id,
                created_at: now.clone(),
                updated_at: now,
                ..Default::default()
            }],
            git: None,
            order: 0,
        }
    }

    fn build_comment_router(state: AppState) -> Router {
        Router::new()
            .route(
                "/api/projects/:id/tasks/:task_id/comment",
                post(add_comment),
            )
            .layer(CorsLayer::permissive())
            .with_state(state)
    }

    // ─── Test 1: Kommentar hinzufügen (Happy Path) ───────────────────────────

    #[tokio::test]
    async fn test_add_comment_happy_path() {
        let (state, _dir) = make_test_state().await;
        let token = create_test_token(&state, "tester", "admin").await;

        let project = make_test_project();
        let task_id = project.tasks[0].id.clone();
        let slug = project.slug.clone();
        state.store.create_project(project).await.unwrap();

        let app = build_comment_router(state.clone());

        let body = serde_json::json!({ "text": "Dies ist ein Testkommentar" }).to_string();
        let req = Request::builder()
            .method(Method::POST)
            .uri(format!("/api/projects/{}/tasks/{}/comment", slug, task_id))
            .header("authorization", format!("Bearer {}", token))
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "POST /comment muss 200 OK zurückgeben"
        );

        // Kommentar muss im Task gespeichert sein
        let updated = state.store.resolve_project(&slug).await.unwrap();
        let task = updated.tasks.iter().find(|t| t.id == task_id).unwrap();
        assert_eq!(
            task.comments.len(),
            1,
            "Task muss genau einen Kommentar haben"
        );
        assert_eq!(
            task.comments[0]["msg"], "Dies ist ein Testkommentar",
            "Kommentar-Text muss übereinstimmen"
        );
    }

    // ─── Test 2: fehlender text → 400 ────────────────────────────────────────

    #[tokio::test]
    async fn test_add_comment_missing_text_returns_400() {
        let (state, _dir) = make_test_state().await;
        let token = create_test_token(&state, "tester", "admin").await;

        let project = make_test_project();
        let task_id = project.tasks[0].id.clone();
        let slug = project.slug.clone();
        state.store.create_project(project).await.unwrap();

        let app = build_comment_router(state);

        // Body ohne "text"-Feld
        let body = serde_json::json!({}).to_string();
        let req = Request::builder()
            .method(Method::POST)
            .uri(format!("/api/projects/{}/tasks/{}/comment", slug, task_id))
            .header("authorization", format!("Bearer {}", token))
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::BAD_REQUEST,
            "Fehlender text muss 400 Bad Request liefern"
        );
    }

    // ─── Test 3: ohne Auth → 401 ─────────────────────────────────────────────

    #[tokio::test]
    async fn test_add_comment_requires_auth() {
        let (state, _dir) = make_test_state().await;

        let project = make_test_project();
        let task_id = project.tasks[0].id.clone();
        let slug = project.slug.clone();
        state.store.create_project(project).await.unwrap();

        let app = build_comment_router(state);

        let body = serde_json::json!({ "text": "Kein Auth" }).to_string();
        let req = Request::builder()
            .method(Method::POST)
            .uri(format!("/api/projects/{}/tasks/{}/comment", slug, task_id))
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "Ohne Auth-Token muss 401 kommen"
        );
    }

    // ─── Test 4: CLI-Script enthält v0.2.0-Subcommands ───────────────────────

    #[test]
    fn test_cli_script_contains_v020_subcommands() {
        let script = build_cli_script("http://localhost:3000");

        assert!(
            script.contains("cmd_task_move"),
            "CLI-Script muss cmd_task_move enthalten"
        );
        assert!(
            script.contains("cmd_task_done"),
            "CLI-Script muss cmd_task_done enthalten"
        );
        assert!(
            script.contains("cmd_task_comment"),
            "CLI-Script muss cmd_task_comment enthalten"
        );
        assert!(
            script.contains("cmd_task_create"),
            "CLI-Script muss cmd_task_create enthalten"
        );

        // Dispatcher muss alle Subcommands routen
        assert!(
            script.contains("move)") || script.contains("\"move\")"),
            "CLI-Script muss 'move' im task-Dispatch haben"
        );
        assert!(
            script.contains("done)") || script.contains("\"done\")"),
            "CLI-Script muss 'done' im task-Dispatch haben"
        );
        assert!(
            script.contains("comment)") || script.contains("\"comment\")"),
            "CLI-Script muss 'comment' im task-Dispatch haben"
        );
        assert!(
            script.contains("create)") || script.contains("\"create\")"),
            "CLI-Script muss 'create' im task-Dispatch haben"
        );
    }

    // ─── Test 5: CLI-Script meldet VERSION 0.2.0 ─────────────────────────────

    #[test]
    fn test_cli_script_version_is_020() {
        let script = build_cli_script("http://localhost:3000");
        assert!(
            script.contains("VERSION=\"0.2.0\"") || script.contains("VERSION='0.2.0'"),
            "CLI-Script muss VERSION=0.2.0 deklarieren, gefunden in Script: {}",
            &script[..script.find("VERSION").unwrap_or(200).min(200)]
        );
    }

    // ─── Test 6: comment-Endpoint nutzt korrekte URL im CLI-Script ───────────

    #[test]
    fn test_cli_script_comment_calls_correct_endpoint() {
        let script = build_cli_script("http://localhost:3000");
        // cmd_task_comment muss /comment aufrufen
        let fn_start = script.find("cmd_task_comment()").unwrap_or(0);
        // suche das nächste Funktionsende (nächste leere Zeile nach "}")
        let fn_end = script[fn_start..]
            .find("\ncmd_task_")
            .map(|i| fn_start + i)
            .unwrap_or(script.len());
        let fn_body = &script[fn_start..fn_end];
        assert!(
            fn_body.contains("/comment"),
            "cmd_task_comment muss POST .../comment aufrufen, gefunden:\n{}",
            &fn_body[..fn_body.len().min(500)]
        );
    }
}
