//! HTTP-Integrationstests für `column_entered_at` via REST API (Ticket 52151531).
//!
//! Prüft auf HTTP/Router-Ebene:
//! 1. Task anlegen (kein `column_entered_at` in Response)
//! 2. Task via REST `POST /api/projects/:id/tasks/:task_id/move` verschieben
//!    → Response `ok: true`, anschließend Projekt neu laden → `column_entered_at` gesetzt
//! 3. Zweites Move → `column_entered_at` aktualisiert (neuer Timestamp)

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use axum::routing::{get, post};
    use axum::Router;
    use tower::ServiceExt;
    use uuid::Uuid;

    use crate::controllers::project_controller::get_project;
    use crate::controllers::task_controller::{create_task, move_task};
    use crate::models::auth::AuthUser;
    use crate::models::project::{Column, ProjectDoc, Task};
    use crate::models::project_slugify;
    use crate::services::auth_service::create_jwt;
    use crate::state::AppState;
    use crate::store::{DataStore, FileStore};

    // ── Hilfsfunktionen ──────────────────────────────────────────────────────

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
            last_maintenance_run: Arc::new(tokio::sync::RwLock::new(None)),
            started_at: chrono::Utc::now(),
            attachment_store: None,
        };
        (state, dir)
    }

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

    fn build_app(state: AppState) -> Router {
        Router::new()
            .route("/api/projects/:id", get(get_project))
            .route("/api/projects/:id/tasks", post(create_task))
            .route("/api/projects/:id/tasks/:task_id/move", post(move_task))
            .with_state(state)
    }

    async fn create_test_project(state: &AppState) -> ProjectDoc {
        let todo_id = Uuid::new_v4().to_string();
        let in_progress_id = Uuid::new_v4().to_string();
        let testing_id = Uuid::new_v4().to_string();
        let project = ProjectDoc {
            id: Uuid::new_v4().to_string(),
            rev: None,
            title: "ColumnEnteredAtHttpTest".to_string(),
            slug: project_slugify("ColumnEnteredAtHttpTest"),
            owner: None,
            webhook_url: None,
            columns: vec![
                Column {
                    id: todo_id,
                    title: "Todo".to_string(),
                    slug: "TODO".to_string(),
                    order: 0,
                    color: "#90CAF9".to_string(),
                    hidden: false,
                    locked: true,
                },
                Column {
                    id: in_progress_id,
                    title: "In Progress".to_string(),
                    slug: "IN_PROGRESS".to_string(),
                    order: 1,
                    color: "#FFF9C4".to_string(),
                    hidden: false,
                    locked: false,
                },
                Column {
                    id: testing_id,
                    title: "Testing".to_string(),
                    slug: "TESTING".to_string(),
                    order: 2,
                    color: "#C8E6C9".to_string(),
                    hidden: false,
                    locked: false,
                },
            ],
            users: vec![],
            tasks: vec![],
            git: None,
            order: 0,
            r#type: None,
            done_expire: None,
            archive_delete: None,
            pinned: None,
        };
        state
            .store
            .create_project(project)
            .await
            .expect("create_project")
    }

    /// GET /api/projects/:id und Task mit angegebener ID zurückgeben.
    async fn reload_task(
        app: Router,
        jwt: &str,
        project_id: &str,
        task_id: &str,
    ) -> serde_json::Value {
        let req = Request::builder()
            .method(Method::GET)
            .uri(format!("/api/projects/{}", project_id))
            .header("authorization", format!("Bearer {}", jwt))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.expect("request");
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "GET project muss 200 liefern"
        );
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let project: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let tasks = project["tasks"].as_array().expect("tasks array");
        tasks
            .iter()
            .find(|t| t["id"].as_str() == Some(task_id))
            .cloned()
            .expect("task not found in project response")
    }

    // ── Test 1: Erster REST move_task setzt column_entered_at ────────────────

    /// POST /api/projects/:id/tasks/:task_id/move → column_entered_at wird gesetzt.
    #[tokio::test]
    async fn test_rest_move_task_sets_column_entered_at() {
        let (state, _dir) = make_test_state().await;
        let jwt = test_jwt(&state.jwt_secret);
        let project = create_test_project(&state).await;
        let project_id = project.id.clone();

        let todo_col_id = project
            .columns
            .iter()
            .find(|c| c.title == "Todo")
            .map(|c| c.id.clone())
            .unwrap();
        let in_progress_col_id = project
            .columns
            .iter()
            .find(|c| c.title == "In Progress")
            .map(|c| c.id.clone())
            .unwrap();

        // Task via REST POST /api/projects/:id/tasks anlegen
        let task_payload = serde_json::json!({
            "title": "Test-Task für column_entered_at",
            "column_id": todo_col_id,
            "creator": "test",
        });
        let app = build_app(state.clone());
        let req = Request::builder()
            .method(Method::POST)
            .uri(format!("/api/projects/{}/tasks", project_id))
            .header("authorization", format!("Bearer {}", jwt))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&task_payload).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.expect("create_task request");
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "create_task muss 200 liefern"
        );
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let project_doc: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let tasks = project_doc["tasks"].as_array().expect("tasks array");
        let created_task = tasks
            .iter()
            .find(|t| t["title"].as_str() == Some("Test-Task für column_entered_at"))
            .expect("Angelegter Task nicht in Response");
        let task_id = created_task["id"].as_str().expect("task id").to_string();

        // column_entered_at darf beim Anlegen noch nicht gesetzt sein
        assert!(
            created_task["column_entered_at"].is_null(),
            "Neuer Task darf noch kein column_entered_at haben, bekommen: {:?}",
            created_task["column_entered_at"]
        );

        // Task via REST move_task verschieben: Todo → In Progress
        let before_move = chrono::Utc::now();
        let move_payload = serde_json::json!({
            "column_id": in_progress_col_id,
        });
        let app2 = build_app(state.clone());
        let req2 = Request::builder()
            .method(Method::POST)
            .uri(format!(
                "/api/projects/{}/tasks/{}/move",
                project_id, task_id
            ))
            .header("authorization", format!("Bearer {}", jwt))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&move_payload).unwrap()))
            .unwrap();
        let resp2 = app2.oneshot(req2).await.expect("move_task request");
        assert_eq!(resp2.status(), StatusCode::OK, "move_task muss 200 liefern");
        let move_bytes = axum::body::to_bytes(resp2.into_body(), usize::MAX)
            .await
            .unwrap();
        let move_resp: serde_json::Value = serde_json::from_slice(&move_bytes).unwrap();
        assert_eq!(
            move_resp["ok"].as_bool(),
            Some(true),
            "move_task Response muss ok:true enthalten"
        );

        // Projekt neu laden und column_entered_at prüfen
        let app3 = build_app(state.clone());
        let updated_task = reload_task(app3, &jwt, &project_id, &task_id).await;

        assert!(
            !updated_task["column_entered_at"].is_null(),
            "column_entered_at muss nach REST move_task gesetzt sein"
        );
        assert_eq!(
            updated_task["column_id"].as_str(),
            Some(in_progress_col_id.as_str()),
            "column_id muss In Progress sein"
        );

        // Timestamp muss >= Zeitpunkt vor dem Move sein
        let entered_at_str = updated_task["column_entered_at"]
            .as_str()
            .expect("column_entered_at als String");
        let entered_at = chrono::DateTime::parse_from_rfc3339(entered_at_str)
            .expect("column_entered_at als RFC3339 parsierbar")
            .with_timezone(&chrono::Utc);
        assert!(
            entered_at >= before_move,
            "column_entered_at ({entered_at}) muss >= Zeitpunkt vor dem Move ({before_move}) sein"
        );
    }

    // ── Test 2: Zweiter REST move_task aktualisiert column_entered_at ────────

    /// Zweites move_task → column_entered_at enthält neuen (späteren) Timestamp.
    #[tokio::test]
    async fn test_rest_move_task_updates_column_entered_at() {
        let (state, _dir) = make_test_state().await;
        let jwt = test_jwt(&state.jwt_secret);
        let project = create_test_project(&state).await;
        let project_id = project.id.clone();

        let todo_col_id = project
            .columns
            .iter()
            .find(|c| c.title == "Todo")
            .map(|c| c.id.clone())
            .unwrap();
        let in_progress_col_id = project
            .columns
            .iter()
            .find(|c| c.title == "In Progress")
            .map(|c| c.id.clone())
            .unwrap();
        let testing_col_id = project
            .columns
            .iter()
            .find(|c| c.title == "Testing")
            .map(|c| c.id.clone())
            .unwrap();

        // Task anlegen
        let task_payload = serde_json::json!({
            "title": "Update-Test column_entered_at",
            "column_id": todo_col_id,
            "creator": "test",
        });
        let app = build_app(state.clone());
        let req = Request::builder()
            .method(Method::POST)
            .uri(format!("/api/projects/{}/tasks", project_id))
            .header("authorization", format!("Bearer {}", jwt))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&task_payload).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let project_doc: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let tasks = project_doc["tasks"].as_array().unwrap();
        let task_id = tasks
            .iter()
            .find(|t| t["title"].as_str() == Some("Update-Test column_entered_at"))
            .expect("task not found")["id"]
            .as_str()
            .unwrap()
            .to_string();

        // Erster Move: Todo → In Progress
        let move1 = serde_json::json!({ "column_id": in_progress_col_id });
        let app2 = build_app(state.clone());
        let req2 = Request::builder()
            .method(Method::POST)
            .uri(format!(
                "/api/projects/{}/tasks/{}/move",
                project_id, task_id
            ))
            .header("authorization", format!("Bearer {}", jwt))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&move1).unwrap()))
            .unwrap();
        app2.oneshot(req2).await.unwrap();

        // Ersten Timestamp lesen
        let app3 = build_app(state.clone());
        let task_after_first = reload_task(app3, &jwt, &project_id, &task_id).await;
        let first_ts_str = task_after_first["column_entered_at"]
            .as_str()
            .expect("column_entered_at nach erstem Move");
        let first_ts = chrono::DateTime::parse_from_rfc3339(first_ts_str)
            .unwrap()
            .with_timezone(&chrono::Utc);

        // Kurze Pause damit der zweite Timestamp unterschiedlich ist
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;

        // Zweiter Move: In Progress → Testing
        let move2 = serde_json::json!({ "column_id": testing_col_id });
        let app4 = build_app(state.clone());
        let req3 = Request::builder()
            .method(Method::POST)
            .uri(format!(
                "/api/projects/{}/tasks/{}/move",
                project_id, task_id
            ))
            .header("authorization", format!("Bearer {}", jwt))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&move2).unwrap()))
            .unwrap();
        app4.oneshot(req3).await.unwrap();

        // Zweiten Timestamp lesen und vergleichen
        let app5 = build_app(state.clone());
        let task_after_second = reload_task(app5, &jwt, &project_id, &task_id).await;
        let second_ts_str = task_after_second["column_entered_at"]
            .as_str()
            .expect("column_entered_at nach zweitem Move");
        let second_ts = chrono::DateTime::parse_from_rfc3339(second_ts_str)
            .unwrap()
            .with_timezone(&chrono::Utc);

        assert!(
            second_ts > first_ts,
            "column_entered_at muss nach zweitem Move neuer sein: {second_ts} > {first_ts}"
        );
        assert_eq!(
            task_after_second["column_id"].as_str(),
            Some(testing_col_id.as_str()),
            "column_id muss Testing sein"
        );
    }
}
