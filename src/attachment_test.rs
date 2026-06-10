//! Integrationstests für File-Attachment-Feature (REST API + MCP).
//! TDD: Tests zuerst rot → Implementierung → grün.

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tokio::sync::Mutex;
    use tower::ServiceExt;

    use crate::models::*;
    use crate::services::attachment_service::MemoryAttachmentStore;
    use crate::services::project_service::default_project;
    use crate::state::AppState;
    use crate::store::{DataStore, FileStore};

    async fn make_test_state_with_attachments() -> (AppState, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = DataStore::File(FileStore {
            root: dir.path().to_path_buf(),
        });
        store.ensure_users_dir().await.ok();

        let attachment_store = Arc::new(MemoryAttachmentStore::new());

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
            attachment_store: Some(attachment_store),
        };
        (state, dir)
    }

    async fn make_test_state_no_s3() -> (AppState, tempfile::TempDir) {
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

    fn build_jwt(secret: &str) -> String {
        use crate::models::AuthUser;
        use crate::services::auth_service::create_jwt;
        let user = AuthUser {
            id: "test-user-id".into(),
            username: "test-user".into(),
            display_name: "test-user".into(),
            role: "admin".into(),
            password_hash: String::new(),
            active: true,
            created_at: String::new(),
            updated_at: String::new(),
        };
        create_jwt(&user, secret, false).expect("jwt")
    }

    // ────────────────────────────────────────────────────────
    // Datenmodell-Tests
    // ────────────────────────────────────────────────────────

    /// Task muss ein `attachments`-Feld haben (Vec<AttachmentRef>).
    #[test]
    fn test_task_has_attachments_field() {
        let task = Task::default();
        assert!(task.attachments.is_empty());
    }

    /// AttachmentRef muss alle Pflichtfelder haben.
    #[test]
    fn test_attachment_ref_serialization() {
        let att = AttachmentRef {
            id: "uuid-1".into(),
            filename: "main.rs".into(),
            url: "https://s3.example.com/bucket/key".into(),
            mime_type: "text/plain".into(),
            size_bytes: 1234,
            created_at: "2026-06-09T00:00:00Z".into(),
        };
        let json = serde_json::to_value(&att).unwrap();
        assert_eq!(json["filename"], "main.rs");
        assert_eq!(json["size_bytes"], 1234);
    }

    /// Attachments sollen beim Serialisieren ausgelassen werden wenn leer (kein `"attachments":[]` Bloat).
    #[test]
    fn test_task_attachments_skip_when_empty() {
        let task = Task {
            id: "t1".into(),
            title: "Test".into(),
            ..Task::default()
        };
        let json = serde_json::to_value(&task).unwrap();
        assert!(
            json.get("attachments").is_none(),
            "empty attachments should be skipped"
        );
    }

    // ────────────────────────────────────────────────────────
    // REST API Tests
    // ────────────────────────────────────────────────────────

    /// POST /api/projects/:id/tasks/:task_id/attachments
    /// → 200 + attachment_id + url zurückgeben.
    #[tokio::test]
    async fn test_upload_attachment_returns_200() {
        let (state, _dir) = make_test_state_with_attachments().await;
        let mut project = default_project("AttachTest".into());
        let task = Task {
            id: "task-1".into(),
            title: "Test Task".into(),
            column_id: project.columns[0].id.clone(),
            ..Task::default()
        };
        project.tasks.push(task);
        let project = state.store.create_project(project).await.unwrap();
        let project_id = project.id.clone();

        let app = crate::build_router(state);

        let jwt = build_jwt("test-secret");
        let boundary = "boundary123";
        let body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"hello.txt\"\r\nContent-Type: text/plain\r\n\r\nhello world\r\n--{boundary}--\r\n"
        );

        let req = Request::builder()
            .method("POST")
            .uri(format!(
                "/api/projects/{}/tasks/task-1/attachments",
                project_id
            ))
            .header("Authorization", format!("Bearer {}", jwt))
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={boundary}"),
            )
            .body(Body::from(body))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(json["id"].as_str().is_some());
        assert!(json["url"].as_str().is_some());
        assert_eq!(json["filename"], "hello.txt");
    }

    /// GET /api/projects/:id/tasks/:task_id/attachments
    /// → Leere Liste wenn keine Anhänge.
    #[tokio::test]
    async fn test_list_attachments_empty() {
        let (state, _dir) = make_test_state_with_attachments().await;
        let mut project = default_project("ListTest".into());
        let task = Task {
            id: "task-1".into(),
            title: "Test".into(),
            column_id: project.columns[0].id.clone(),
            ..Task::default()
        };
        project.tasks.push(task);
        let project = state.store.create_project(project).await.unwrap();

        let app = crate::build_router(state);
        let jwt = build_jwt("test-secret");

        let req = Request::builder()
            .method("GET")
            .uri(format!(
                "/api/projects/{}/tasks/task-1/attachments",
                project.id
            ))
            .header("Authorization", format!("Bearer {}", jwt))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(json.as_array().unwrap().is_empty());
    }

    /// DELETE /api/projects/:id/tasks/:task_id/attachments/:attachment_id → 200.
    #[tokio::test]
    async fn test_delete_attachment() {
        let (state, _dir) = make_test_state_with_attachments().await;
        let attachment_id = "att-1".to_string();
        let mut project = default_project("DelTest".into());
        let task = Task {
            id: "task-1".into(),
            title: "Test".into(),
            column_id: project.columns[0].id.clone(),
            attachments: vec![AttachmentRef {
                id: attachment_id.clone(),
                filename: "file.rs".into(),
                url: "http://memory-store/key".into(),
                mime_type: "text/plain".into(),
                size_bytes: 42,
                created_at: "2026-06-09T00:00:00Z".into(),
            }],
            ..Task::default()
        };
        project.tasks.push(task);
        let project = state.store.create_project(project).await.unwrap();

        let app = crate::build_router(state);
        let jwt = build_jwt("test-secret");

        let req = Request::builder()
            .method("DELETE")
            .uri(format!(
                "/api/projects/{}/tasks/task-1/attachments/{}",
                project.id, attachment_id
            ))
            .header("Authorization", format!("Bearer {}", jwt))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    /// Ohne S3-Config → Attachment-Endpunkte liefern 404 (nicht registriert).
    #[tokio::test]
    async fn test_attachment_routes_disabled_without_s3() {
        let (state, _dir) = make_test_state_no_s3().await;
        let mut project = default_project("NoS3Test".into());
        let task = Task {
            id: "task-1".into(),
            title: "Test".into(),
            column_id: project.columns[0].id.clone(),
            ..Task::default()
        };
        project.tasks.push(task);
        let project = state.store.create_project(project).await.unwrap();

        let app = crate::build_router(state);
        let jwt = build_jwt("test-secret");

        let req = Request::builder()
            .method("GET")
            .uri(format!(
                "/api/projects/{}/tasks/task-1/attachments",
                project.id
            ))
            .header("Authorization", format!("Bearer {}", jwt))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    // ────────────────────────────────────────────────────────
    // MCP Tool Tests
    // ────────────────────────────────────────────────────────

    /// MCP attach_file mit base64-Inhalt → attachment_id + url.
    #[tokio::test]
    async fn test_mcp_attach_file() {
        use base64::Engine;

        let (state, _dir) = make_test_state_with_attachments().await;
        let mut project = default_project("McpAttach".into());
        let task = Task {
            id: "task-mcp".into(),
            title: "MCP Task".into(),
            column_id: project.columns[0].id.clone(),
            ..Task::default()
        };
        project.tasks.push(task);
        let project = state.store.create_project(project).await.unwrap();

        let content_b64 = base64::engine::general_purpose::STANDARD.encode(b"fn main() {}");

        let args = serde_json::json!({
            "project_id": project.id,
            "task_id": "task-mcp",
            "filename": "main.rs",
            "content_base64": content_b64,
            "mime_type": "text/plain"
        });

        let val = crate::controllers::mcp_controller::execute_tool_pub(
            &state,
            "attach_file",
            &args,
            "admin",
        )
        .await
        .expect("attach_file must succeed");

        assert!(val["id"].as_str().is_some());
        assert!(val["url"].as_str().is_some());
    }

    /// MCP attach_file ohne S3 → Fehler "File uploads not configured".
    #[tokio::test]
    async fn test_mcp_attach_file_no_s3() {
        use base64::Engine;

        let (state, _dir) = make_test_state_no_s3().await;
        let mut project = default_project("NoS3Mcp".into());
        let task = Task {
            id: "task-1".into(),
            title: "T".into(),
            column_id: project.columns[0].id.clone(),
            ..Task::default()
        };
        project.tasks.push(task);
        let project = state.store.create_project(project).await.unwrap();

        let content_b64 = base64::engine::general_purpose::STANDARD.encode(b"data");
        let args = serde_json::json!({
            "project_id": project.id,
            "task_id": "task-1",
            "filename": "f.txt",
            "content_base64": content_b64,
        });

        let result = crate::controllers::mcp_controller::execute_tool_pub(
            &state,
            "attach_file",
            &args,
            "admin",
        )
        .await;

        assert!(result.is_err(), "must fail without S3");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("not configured"),
            "error must mention 'not configured': {err}"
        );
    }

    /// MCP attach_file mit zu großer Datei (> 500 KB) → Fehler.
    #[tokio::test]
    async fn test_mcp_attach_file_too_large() {
        use base64::Engine;

        let (state, _dir) = make_test_state_with_attachments().await;
        let mut project = default_project("TooLarge".into());
        let task = Task {
            id: "task-1".into(),
            title: "T".into(),
            column_id: project.columns[0].id.clone(),
            ..Task::default()
        };
        project.tasks.push(task);
        let project = state.store.create_project(project).await.unwrap();

        // 501 KB → über dem 500 KB Limit
        let big_data = vec![0u8; 501 * 1024];
        let content_b64 = base64::engine::general_purpose::STANDARD.encode(&big_data);

        let args = serde_json::json!({
            "project_id": project.id,
            "task_id": "task-1",
            "filename": "big.bin",
            "content_base64": content_b64,
        });

        let result = crate::controllers::mcp_controller::execute_tool_pub(
            &state,
            "attach_file",
            &args,
            "admin",
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("500 KB"),
            "error must mention size limit: {err}"
        );
    }
}
