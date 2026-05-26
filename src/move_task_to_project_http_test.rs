//! HTTP-Integration-Tests für das MCP-Tool `move_task_to_project`.
//! Ticket e8f3834a: Backend-Endpoint „move task to project"
//!
//! Diese Tests rufen den echten POST /mcp/call Endpunkt über den Axum-Router auf
//! (tower::ServiceExt::oneshot) und verifizieren das Verhalten auf API-Ebene.
//!
//! Abgedeckt:
//! 1. Spalten-Mapping Match: Task landet in der gleichnamigen Spalte im Ziel.
//! 2. Spalten-Mapping Fallback: Task landet in der ersten Spalte (order=0) des Ziels.
//! 3. Task aus Quelle entfernt, im Ziel vorhanden (end-to-end Verschiebung).
//! 4. Guard: Selbes Projekt → HTTP 400.
//! 5. Guard: Ziel-Projekt ohne Spalten → HTTP 400.

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
    use uuid::Uuid;

    use crate::controllers::mcp_controller::call_tool;
    use crate::models::auth::AuthUser;
    use crate::models::project::{Column, ProjectDoc, Task};
    use crate::models::project_slugify;
    use crate::services::auth_service::create_jwt;
    use crate::state::AppState;
    use crate::store::{DataStore, FileStore};

    // -----------------------------------------------------------------------
    // Test-Infrastruktur
    // -----------------------------------------------------------------------

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
        };
        (state, dir)
    }

    fn make_app(state: AppState) -> Router {
        Router::new()
            .route("/mcp/call", post(call_tool))
            .with_state(state)
    }

    /// Erstellt ein Admin-JWT für den Test-Benutzer.
    fn test_jwt(secret: &str) -> String {
        let now = chrono::Utc::now().to_rfc3339();
        let user = AuthUser {
            id: "test-user".into(),
            username: "test".into(),
            display_name: "Test User".into(),
            password_hash: "".into(),
            role: "admin".into(),
            created_at: now.clone(),
            updated_at: now,
            active: true,
        };
        create_jwt(&user, secret, false).expect("create_jwt")
    }

    /// Sendet einen POST /mcp/call mit dem angegebenen Tool und Argumenten.
    async fn call_mcp_tool(
        app: Router,
        jwt: &str,
        tool: &str,
        args: serde_json::Value,
    ) -> (StatusCode, serde_json::Value) {
        let body = serde_json::json!({
            "tool": tool,
            "arguments": args
        });
        let req = Request::builder()
            .method("POST")
            .uri("/mcp/call")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {jwt}"))
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.expect("request");
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let json: serde_json::Value =
            serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
        (status, json)
    }

    /// Erstellt ein Projekt mit den angegebenen Spaltentiteln im Store.
    async fn create_project_with_columns(
        state: &AppState,
        title: &str,
        col_titles: &[&str],
    ) -> ProjectDoc {
        let project = ProjectDoc {
            id: Uuid::new_v4().to_string(),
            rev: None,
            title: title.to_string(),
            slug: project_slugify(title),
            owner: None,
            columns: col_titles
                .iter()
                .enumerate()
                .map(|(i, t)| Column {
                    id: Uuid::new_v4().to_string(),
                    title: t.to_string(),
                    slug: t.to_uppercase().replace(' ', "_"),
                    order: i as i32,
                    color: "#ccc".into(),
                    hidden: false,
                    locked: false,
                })
                .collect(),
            users: vec![],
            tasks: vec![],
            git: None,
            webhook_url: None,
            order: 0,
            r#type: None,
            done_expire: None,
            archive_delete: None,
        };
        state
            .store
            .create_project(project.clone())
            .await
            .expect("create_project")
    }

    /// Fügt einen Task in eine Spalte eines Projekts ein.
    async fn add_task_to_project(
        state: &AppState,
        project_id: &str,
        col_id: &str,
        title: &str,
    ) -> Task {
        let mut project = state
            .store
            .get_project(project_id)
            .await
            .expect("get_project");
        let now = chrono::Utc::now().to_rfc3339();
        let task = Task {
            id: Uuid::new_v4().to_string(),
            title: title.to_string(),
            column_id: col_id.to_string(),
            creator: "test".to_string(),
            created_at: now.clone(),
            updated_at: now,
            ..Task::default()
        };
        project.tasks.push(task.clone());
        state.store.put_project(project).await.expect("put_project");
        task
    }

    // -----------------------------------------------------------------------
    // Test 1: Spalten-Mapping – Match gefunden
    // -----------------------------------------------------------------------

    /// HTTP-Test: Task in "In Progress" → Ziel hat "In Progress" → landet dort.
    #[tokio::test]
    async fn http_test_move_task_column_match() {
        let (state, _dir) = make_test_state().await;
        let jwt = test_jwt(&state.jwt_secret);

        let src =
            create_project_with_columns(&state, "Source", &["Todo", "In Progress", "Done"]).await;
        let dst =
            create_project_with_columns(&state, "Destination", &["Todo", "In Progress", "Done"])
                .await;

        let src_in_progress = src
            .columns
            .iter()
            .find(|c| c.title == "In Progress")
            .unwrap();
        let task =
            add_task_to_project(&state, &src.id, &src_in_progress.id, "HTTP Match Task").await;

        let app = make_app(state.clone());
        let (status, body) = call_mcp_tool(
            app,
            &jwt,
            "move_task_to_project",
            serde_json::json!({
                "task_id": task.id,
                "source_project_id": src.id,
                "target_project_id": dst.id
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK, "Erwartet HTTP 200, Body: {body}");

        let new_task_id = body["task_id"].as_str().expect("task_id in response");
        let col_id = body["column_id"].as_str().expect("column_id in response");

        // Zielspalte muss "In Progress" im Ziel sein
        let dst_state = state.store.get_project(&dst.id).await.expect("get dst");
        let dst_in_progress = dst_state
            .columns
            .iter()
            .find(|c| c.title == "In Progress")
            .expect("In Progress nicht im Zielprojekt");
        assert_eq!(
            col_id, dst_in_progress.id,
            "Task soll in 'In Progress' des Ziels landen"
        );

        // Task im Zielprojekt vorhanden
        assert!(
            dst_state.tasks.iter().any(|t| t.id == new_task_id),
            "Neuer Task muss im Zielprojekt existieren"
        );

        // Task aus Quellprojekt entfernt
        let src_state = state.store.get_project(&src.id).await.expect("get src");
        assert!(
            !src_state.tasks.iter().any(|t| t.id == task.id),
            "Alter Task muss aus Quellprojekt entfernt sein"
        );
    }

    // -----------------------------------------------------------------------
    // Test 2: Spalten-Mapping – Kein Match → erste Spalte (order=0)
    // -----------------------------------------------------------------------

    /// HTTP-Test: Task in "Custom Column" → kein Match im Ziel → erste Spalte.
    #[tokio::test]
    async fn http_test_move_task_column_fallback() {
        let (state, _dir) = make_test_state().await;
        let jwt = test_jwt(&state.jwt_secret);

        let src = create_project_with_columns(&state, "Source", &["Todo", "Custom Column"]).await;
        let dst =
            create_project_with_columns(&state, "Destination", &["Backlog", "Active", "Done"])
                .await;

        let src_custom = src
            .columns
            .iter()
            .find(|c| c.title == "Custom Column")
            .unwrap();
        let task = add_task_to_project(&state, &src.id, &src_custom.id, "Fallback Task HTTP").await;

        let app = make_app(state.clone());
        let (status, body) = call_mcp_tool(
            app,
            &jwt,
            "move_task_to_project",
            serde_json::json!({
                "task_id": task.id,
                "source_project_id": src.id,
                "target_project_id": dst.id
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK, "Erwartet HTTP 200, Body: {body}");

        let col_id = body["column_id"].as_str().expect("column_id in response");

        // Erste Spalte (order=0) des Zielprojekts
        let dst_state = state.store.get_project(&dst.id).await.expect("get dst");
        let first_col = dst_state
            .columns
            .iter()
            .min_by_key(|c| c.order)
            .expect("keine Spalte im Ziel");
        assert_eq!(
            col_id, first_col.id,
            "Fallback: Task muss in der ersten Spalte (order=0) landen"
        );
    }

    // -----------------------------------------------------------------------
    // Test 3: End-to-End – Task aus Quelle weg, im Ziel vorhanden
    // -----------------------------------------------------------------------

    /// HTTP-Test: Vollständige Verschiebung, inkl. Kommentare/Logs-Erhalt.
    #[tokio::test]
    async fn http_test_move_task_source_removed_target_present() {
        let (state, _dir) = make_test_state().await;
        let jwt = test_jwt(&state.jwt_secret);

        let src = create_project_with_columns(&state, "Source E2E", &["Todo"]).await;
        let dst = create_project_with_columns(&state, "Dest E2E", &["Todo", "Done"]).await;

        // Task mit Kommentar und Log anlegen
        let src_col = &src.columns[0];
        let mut project = state.store.get_project(&src.id).await.expect("get src");
        let now = chrono::Utc::now().to_rfc3339();
        let task = Task {
            id: Uuid::new_v4().to_string(),
            title: "E2E Task".to_string(),
            column_id: src_col.id.clone(),
            creator: "tester".to_string(),
            created_at: now.clone(),
            updated_at: now,
            comments: vec![
                serde_json::json!({"user": "tester", "msg": "wichtiger Kommentar", "ts": "01-01 10:00"}),
            ],
            logs: vec![
                serde_json::json!({"user": "system", "msg": "erstellt", "ts": "01-01 09:00"}),
            ],
            ..Task::default()
        };
        let task_id = task.id.clone();
        project.tasks.push(task);
        state.store.put_project(project).await.expect("put_project");

        let app = make_app(state.clone());
        let (status, body) = call_mcp_tool(
            app,
            &jwt,
            "move_task_to_project",
            serde_json::json!({
                "task_id": task_id,
                "source_project_id": src.id,
                "target_project_id": dst.id
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK, "Erwartet HTTP 200, Body: {body}");

        let new_task_id = body["task_id"].as_str().expect("task_id in response");

        // Quellprojekt: Task entfernt
        let src_state = state.store.get_project(&src.id).await.expect("get src");
        assert!(
            !src_state.tasks.iter().any(|t| t.id == task_id),
            "Task muss aus Quellprojekt entfernt sein"
        );

        // Zielprojekt: Task vorhanden
        let dst_state = state.store.get_project(&dst.id).await.expect("get dst");
        let moved = dst_state
            .tasks
            .iter()
            .find(|t| t.id == new_task_id)
            .expect("Task nicht im Zielprojekt");

        assert_eq!(moved.title, "E2E Task", "Titel muss erhalten bleiben");
        assert_eq!(moved.creator, "tester", "Creator muss erhalten bleiben");
        assert!(
            !moved.comments.is_empty(),
            "Kommentare müssen erhalten bleiben"
        );
        // Das Log enthält mind. den ursprünglichen + den neuen "moved from"-Eintrag
        assert!(moved.logs.len() >= 2, "Logs müssen erhalten bleiben");
    }

    // -----------------------------------------------------------------------
    // Test 4: Guard – selbes Projekt → HTTP 400
    // -----------------------------------------------------------------------

    /// HTTP-Test: Verschieben ins selbe Projekt → 400 Bad Request.
    #[tokio::test]
    async fn http_test_move_task_same_project_guard() {
        let (state, _dir) = make_test_state().await;
        let jwt = test_jwt(&state.jwt_secret);

        let project = create_project_with_columns(&state, "Same Project", &["Todo", "Done"]).await;
        let col = &project.columns[0];
        let task = add_task_to_project(&state, &project.id, &col.id, "Guard Task").await;

        let app = make_app(state.clone());
        let (status, _body) = call_mcp_tool(
            app,
            &jwt,
            "move_task_to_project",
            serde_json::json!({
                "task_id": task.id,
                "source_project_id": project.id,
                "target_project_id": project.id
            }),
        )
        .await;

        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "Verschieben ins selbe Projekt muss HTTP 400 liefern"
        );
    }

    // -----------------------------------------------------------------------
    // Test 5: Guard – Ziel ohne Spalten → HTTP 400
    // -----------------------------------------------------------------------

    /// HTTP-Test: Ziel-Projekt ohne Spalten → 400 Bad Request.
    #[tokio::test]
    async fn http_test_move_task_target_no_columns() {
        let (state, _dir) = make_test_state().await;
        let jwt = test_jwt(&state.jwt_secret);

        let src = create_project_with_columns(&state, "Source No-Col", &["Todo"]).await;

        // Ziel ohne Spalten
        let dst = ProjectDoc {
            id: Uuid::new_v4().to_string(),
            rev: None,
            title: "Empty Dest".to_string(),
            slug: "empty-dest".to_string(),
            owner: None,
            columns: vec![],
            users: vec![],
            tasks: vec![],
            git: None,
            webhook_url: None,
            order: 0,
            r#type: None,
            done_expire: None,
            archive_delete: None,
        };
        let dst = state.store.create_project(dst).await.expect("create dst");

        let src_col = &src.columns[0];
        let task = add_task_to_project(&state, &src.id, &src_col.id, "No-Columns Task").await;

        let app = make_app(state.clone());
        let (status, _body) = call_mcp_tool(
            app,
            &jwt,
            "move_task_to_project",
            serde_json::json!({
                "task_id": task.id,
                "source_project_id": src.id,
                "target_project_id": dst.id
            }),
        )
        .await;

        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "Ziel ohne Spalten muss HTTP 400 liefern"
        );
    }
}
