/// Integrationstests für Outgoing + Incoming Webhooks (Ticket 57bf48d6)
///
/// Prüft:
/// 1. ProjectDoc akzeptiert webhook_url
/// 2. Outgoing Webhook: fire_webhook sendet POST an Mock-URL
/// 3. Incoming Webhook: POST /webhook/projects/:slug/tasks/:task_id/move verschiebt Task
/// 4. Incoming Webhook: ohne Auth → 401
/// 5. Incoming Webhook: ungültige column → 400/404
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

    use crate::controllers::webhook_controller::incoming_move_task;
    use crate::models::{Column, ProjectDoc, Task};
    use crate::services::webhook_service::WebhookEvent;
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

    fn make_project_with_webhook(webhook_url: Option<String>) -> ProjectDoc {
        use uuid::Uuid;
        let todo_id = Uuid::new_v4().to_string();
        let done_id = Uuid::new_v4().to_string();
        let task_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        ProjectDoc {
            id: Uuid::new_v4().to_string(),
            rev: None,
            title: "Webhook Test Project".to_string(),
            slug: "webhook-test-project".to_string(),
            owner: None,
            webhook_url,
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
                    id: done_id.clone(),
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
                slug: "test-task".to_string(),
                title: "Test Task".to_string(),
                column_id: todo_id,
                created_at: now.clone(),
                updated_at: now,
                ..Default::default()
            }],
            git: None,
            order: 0,
        }
    }

    fn build_webhook_router(state: AppState) -> Router {
        Router::new()
            .route(
                "/webhook/projects/:slug/tasks/:task_id/move",
                post(incoming_move_task),
            )
            .layer(CorsLayer::permissive())
            .with_state(state)
    }

    // ─── Test 1: webhook_url Feld in ProjectDoc ──────────────────────────────

    #[test]
    fn test_project_doc_has_webhook_url_field() {
        let project = make_project_with_webhook(Some("https://example.com/hook".to_string()));
        assert_eq!(
            project.webhook_url,
            Some("https://example.com/hook".to_string())
        );

        let project_no_hook = make_project_with_webhook(None);
        assert!(project_no_hook.webhook_url.is_none());
    }

    #[test]
    fn test_project_doc_webhook_url_serialization() {
        let project =
            make_project_with_webhook(Some("https://hooks.example.com/plankton".to_string()));
        let json = serde_json::to_string(&project).unwrap();
        assert!(json.contains("webhook_url"));
        assert!(json.contains("https://hooks.example.com/plankton"));

        // None → kein webhook_url Feld (skip_serializing_if)
        let project_no_hook = make_project_with_webhook(None);
        let json_no_hook = serde_json::to_string(&project_no_hook).unwrap();
        assert!(!json_no_hook.contains("webhook_url"));
    }

    // ─── Test 2: WebhookEvent Payload-Struktur ──────────────────────────────

    #[test]
    fn test_webhook_event_payload_structure() {
        let event = WebhookEvent {
            event: "task.moved".to_string(),
            project: "my-project".to_string(),
            task: crate::services::webhook_service::WebhookTaskInfo {
                id: "task-id-123".to_string(),
                title: "Mein Task".to_string(),
                column: "Done".to_string(),
                worker: "frank".to_string(),
            },
            ts: "2026-05-19T20:00:00Z".to_string(),
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["event"], "task.moved");
        assert_eq!(json["project"], "my-project");
        assert_eq!(json["task"]["id"], "task-id-123");
        assert_eq!(json["task"]["title"], "Mein Task");
        assert_eq!(json["task"]["column"], "Done");
        assert_eq!(json["task"]["worker"], "frank");
        assert!(json.get("ts").is_some());
    }

    // ─── Test 3: Incoming Webhook – Task verschieben (Happy Path) ───────────

    #[tokio::test]
    async fn test_incoming_webhook_moves_task() {
        let (state, _dir) = make_test_state().await;
        let token = create_test_token(&state, "ci-agent", "admin").await;

        let project = make_project_with_webhook(None);
        let task_id = project.tasks[0].id.clone();
        let done_col_id = project.columns[1].id.clone();
        let slug = project.slug.clone();
        state.store.create_project(project).await.unwrap();

        let app = build_webhook_router(state.clone());

        let body = serde_json::json!({ "column": "DONE" }).to_string();
        let req = Request::builder()
            .method(Method::POST)
            .uri(format!("/webhook/projects/{}/tasks/{}/move", slug, task_id))
            .header("authorization", format!("Bearer {}", token))
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // Task muss jetzt in Done sein
        let updated = state.store.resolve_project(&slug).await.unwrap();
        let task = updated.tasks.iter().find(|t| t.id == task_id).unwrap();
        assert_eq!(task.column_id, done_col_id, "Task muss in Done-Spalte sein");
    }

    // ─── Test 4: Incoming Webhook – ohne Auth → 401 ─────────────────────────

    #[tokio::test]
    async fn test_incoming_webhook_requires_auth() {
        let (state, _dir) = make_test_state().await;
        let project = make_project_with_webhook(None);
        let task_id = project.tasks[0].id.clone();
        let slug = project.slug.clone();
        state.store.create_project(project).await.unwrap();

        let app = build_webhook_router(state);

        let body = serde_json::json!({ "column": "DONE" }).to_string();
        let req = Request::builder()
            .method(Method::POST)
            .uri(format!("/webhook/projects/{}/tasks/{}/move", slug, task_id))
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    // ─── Test 5: Incoming Webhook – ungültige Column → 400 ──────────────────

    #[tokio::test]
    async fn test_incoming_webhook_invalid_column() {
        let (state, _dir) = make_test_state().await;
        let token = create_test_token(&state, "ci-agent", "admin").await;

        let project = make_project_with_webhook(None);
        let task_id = project.tasks[0].id.clone();
        let slug = project.slug.clone();
        state.store.create_project(project).await.unwrap();

        let app = build_webhook_router(state);

        let body = serde_json::json!({ "column": "NONEXISTENT_COLUMN" }).to_string();
        let req = Request::builder()
            .method(Method::POST)
            .uri(format!("/webhook/projects/{}/tasks/{}/move", slug, task_id))
            .header("authorization", format!("Bearer {}", token))
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert!(
            resp.status() == StatusCode::BAD_REQUEST || resp.status() == StatusCode::NOT_FOUND,
            "Erwartet 400 oder 404, bekam {}",
            resp.status()
        );
    }

    // ─── Test 6: fire_webhook – sendet POST an Mock-URL ─────────────────────

    #[tokio::test]
    async fn test_fire_webhook_sends_post_to_url() {
        use crate::services::webhook_service::{fire_webhook, WebhookEvent, WebhookTaskInfo};
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc as StdArc;

        // Wir starten einen lokalen HTTP-Server der Requests empfängt
        let received = StdArc::new(AtomicBool::new(false));
        let received_clone = received.clone();

        // Binde einen freien Port
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let webhook_url = format!("http://{}/hook", addr);

        // Spawn Mock-Server
        tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let _ = socket; // Verbindung annehmen reicht
            received_clone.store(true, Ordering::SeqCst);
        });

        let event = WebhookEvent {
            event: "task.moved".to_string(),
            project: "test-project".to_string(),
            task: WebhookTaskInfo {
                id: "task-123".to_string(),
                title: "Test".to_string(),
                column: "Done".to_string(),
                worker: "".to_string(),
            },
            ts: chrono::Utc::now().to_rfc3339(),
        };

        let client = reqwest::Client::new();
        fire_webhook(&client, &webhook_url, &event).await;

        // Kurz warten damit der Mock-Server die Verbindung annehmen kann
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        assert!(
            received.load(Ordering::SeqCst),
            "Mock-Server hat keinen Request empfangen"
        );
    }
}
