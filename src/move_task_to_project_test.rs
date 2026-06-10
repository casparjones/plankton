//! Tests für den MCP-Tool `move_task_to_project`
//! Ticket e8f3834a: Backend-Endpoint „move task to project"
//!
//! Prüft:
//! 1. Unit-Test Spalten-Mapping: Match → gleiche Spalte im Ziel
//! 2. Unit-Test Spalten-Mapping: Kein Match → erste Spalte (order=0) des Zielprojekts
//! 3. Unit-Test: Ziel-Projekt ohne Spalten → Error
//! 4. Unit-Test: Verschieben ins selbe Projekt → Error (Guard)
//! 5. Integration-Test: Task erscheint im Zielprojekt, ist im Quellprojekt weg
//! 6. Integration-Test: Relations/Kommentare/Logs bleiben erhalten

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    use crate::controllers::mcp_controller::execute_tool_pub;
    use crate::models::project::{Column, ProjectDoc, Task};
    use crate::models::project_slugify;
    use crate::state::AppState;
    use crate::store::{DataStore, FileStore};
    use uuid::Uuid;

    // -----------------------------------------------------------------------
    // Hilfsfunktionen
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
            last_maintenance_run: Arc::new(tokio::sync::RwLock::new(None)),
            started_at: chrono::Utc::now(),
            attachment_store: None,
        };
        (state, dir)
    }

    /// Erstellt ein Projekt mit angegebenen Spaltentiteln im Store.
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
            pinned: None,
        };
        state
            .store
            .create_project(project.clone())
            .await
            .expect("create_project")
    }

    /// Fügt einen Task in ein Projekt ein und speichert es.
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

    /// Task befindet sich in "In Progress" → Zielprojekt hat auch "In Progress" → landet dort.
    #[tokio::test]
    async fn test_move_task_column_match() {
        let (state, _dir) = make_test_state().await;

        let src =
            create_project_with_columns(&state, "Source Project", &["Todo", "In Progress", "Done"])
                .await;
        let dst = create_project_with_columns(
            &state,
            "Destination Project",
            &["Todo", "In Progress", "Done"],
        )
        .await;

        // Task in "In Progress"-Spalte des Quellprojekts
        let src_in_progress = src
            .columns
            .iter()
            .find(|c| c.title == "In Progress")
            .unwrap();
        let task = add_task_to_project(&state, &src.id, &src_in_progress.id, "My Task").await;

        let args = serde_json::json!({
            "task_id": task.id,
            "source_project_id": src.id,
            "target_project_id": dst.id
        });

        let result = execute_tool_pub(&state, "move_task_to_project", &args, "test")
            .await
            .expect("move_task_to_project should succeed");

        // Response enthält neue task_id + target_column_id
        assert!(
            !result["task_id"].as_str().unwrap_or("").is_empty(),
            "task_id fehlt"
        );
        let target_col_id = result["column_id"].as_str().expect("column_id fehlt");

        // Zielspalte muss "In Progress" im Ziel sein
        let dst_updated = state.store.get_project(&dst.id).await.expect("get dst");
        let dst_in_progress = dst_updated
            .columns
            .iter()
            .find(|c| c.title == "In Progress")
            .expect("In Progress nicht im Zielprojekt");
        assert_eq!(
            target_col_id, dst_in_progress.id,
            "Task soll in 'In Progress' landen"
        );

        // Task ist im Zielprojekt vorhanden
        let new_task_id = result["task_id"].as_str().unwrap();
        let dst_task = dst_updated
            .tasks
            .iter()
            .find(|t| t.id == new_task_id)
            .expect("Task nicht im Zielprojekt gefunden");
        assert_eq!(dst_task.title, "My Task");
    }

    // -----------------------------------------------------------------------
    // Test 2: Spalten-Mapping – Kein Match → erste Spalte (order=0)
    // -----------------------------------------------------------------------

    /// Task in "Custom Column" → nicht im Ziel vorhanden → landet in erster Spalte.
    #[tokio::test]
    async fn test_move_task_column_fallback_to_first() {
        let (state, _dir) = make_test_state().await;

        let src =
            create_project_with_columns(&state, "Source Project", &["Todo", "Custom Column"]).await;
        let dst = create_project_with_columns(
            &state,
            "Destination Project",
            &["Backlog", "Active", "Complete"],
        )
        .await;

        // Task in "Custom Column" – kein Match im Ziel
        let src_custom = src
            .columns
            .iter()
            .find(|c| c.title == "Custom Column")
            .unwrap();
        let task = add_task_to_project(&state, &src.id, &src_custom.id, "Fallback Task").await;

        let args = serde_json::json!({
            "task_id": task.id,
            "source_project_id": src.id,
            "target_project_id": dst.id
        });

        let result = execute_tool_pub(&state, "move_task_to_project", &args, "test")
            .await
            .expect("move_task_to_project should succeed");

        let target_col_id = result["column_id"].as_str().expect("column_id fehlt");

        // Erste Spalte (order=0) im Zielprojekt
        let dst_updated = state.store.get_project(&dst.id).await.expect("get dst");
        let first_col = dst_updated
            .columns
            .iter()
            .min_by_key(|c| c.order)
            .expect("keine Spalte im Ziel");
        assert_eq!(
            target_col_id, first_col.id,
            "Fallback muss erste Spalte (order=0) sein"
        );
    }

    // -----------------------------------------------------------------------
    // Test 3: Ziel-Projekt ohne Spalten → Error
    // -----------------------------------------------------------------------

    /// Wenn Ziel-Projekt keine Spalten hat, muss ein Fehler zurückgegeben werden.
    #[tokio::test]
    async fn test_move_task_target_no_columns_error() {
        let (state, _dir) = make_test_state().await;

        let src = create_project_with_columns(&state, "Source Project", &["Todo"]).await;

        // Ziel ohne Spalten
        let dst = ProjectDoc {
            id: Uuid::new_v4().to_string(),
            rev: None,
            title: "Empty Project".to_string(),
            slug: "empty-project".to_string(),
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
            pinned: None,
        };
        let dst = state.store.create_project(dst).await.expect("create dst");

        let src_col = &src.columns[0];
        let task = add_task_to_project(&state, &src.id, &src_col.id, "Task").await;

        let args = serde_json::json!({
            "task_id": task.id,
            "source_project_id": src.id,
            "target_project_id": dst.id
        });

        let result = execute_tool_pub(&state, "move_task_to_project", &args, "test").await;
        assert!(
            result.is_err(),
            "Sollte Fehler bei leerem Ziel-Projekt zurückgeben"
        );
    }

    // -----------------------------------------------------------------------
    // Test 4: Guard – selbes Projekt
    // -----------------------------------------------------------------------

    /// Verschieben in dasselbe Projekt soll einen Fehler liefern.
    #[tokio::test]
    async fn test_move_task_same_project_guard() {
        let (state, _dir) = make_test_state().await;

        let project = create_project_with_columns(&state, "Same Project", &["Todo", "Done"]).await;
        let col = &project.columns[0];
        let task = add_task_to_project(&state, &project.id, &col.id, "Task").await;

        let args = serde_json::json!({
            "task_id": task.id,
            "source_project_id": project.id,
            "target_project_id": project.id
        });

        let result = execute_tool_pub(&state, "move_task_to_project", &args, "test").await;
        assert!(
            result.is_err(),
            "Verschieben ins selbe Projekt soll Fehler liefern"
        );
    }

    // -----------------------------------------------------------------------
    // Test 5: Integration – Task im Ziel, weg aus Quelle
    // -----------------------------------------------------------------------

    /// Nach dem Verschieben: Task ist im Zielprojekt, im Quellprojekt nicht mehr vorhanden.
    #[tokio::test]
    async fn test_move_task_source_removed_target_present() {
        let (state, _dir) = make_test_state().await;

        let src = create_project_with_columns(&state, "Source", &["Todo", "In Progress"]).await;
        let dst =
            create_project_with_columns(&state, "Destination", &["Todo", "In Progress", "Done"])
                .await;

        let src_col = &src.columns[0];
        let task = add_task_to_project(&state, &src.id, &src_col.id, "Important Task").await;
        let original_task_id = task.id.clone();

        let args = serde_json::json!({
            "task_id": task.id,
            "source_project_id": src.id,
            "target_project_id": dst.id
        });

        execute_tool_pub(&state, "move_task_to_project", &args, "test")
            .await
            .expect("move_task_to_project should succeed");

        // Quellprojekt: Task weg
        let src_updated = state.store.get_project(&src.id).await.expect("get src");
        assert!(
            !src_updated.tasks.iter().any(|t| t.id == original_task_id),
            "Task muss aus Quellprojekt entfernt sein"
        );

        // Zielprojekt: Task vorhanden (unter neuer oder gleicher ID)
        let dst_updated = state.store.get_project(&dst.id).await.expect("get dst");
        assert!(
            dst_updated
                .tasks
                .iter()
                .any(|t| t.title == "Important Task"),
            "Task muss im Zielprojekt vorhanden sein"
        );
    }

    // -----------------------------------------------------------------------
    // Test 6: Relations/Kommentare/Logs bleiben erhalten
    // -----------------------------------------------------------------------

    /// Nach dem Verschieben bleiben Kommentare und Logs des Tasks erhalten.
    #[tokio::test]
    async fn test_move_task_preserves_comments_and_logs() {
        let (state, _dir) = make_test_state().await;

        let src = create_project_with_columns(&state, "Source", &["Todo"]).await;
        let dst = create_project_with_columns(&state, "Dest", &["Todo", "Done"]).await;

        // Task mit Kommentar und Log einfügen
        let src_col = &src.columns[0];
        let mut project = state.store.get_project(&src.id).await.expect("get src");
        let now = chrono::Utc::now().to_rfc3339();
        let task = Task {
            id: Uuid::new_v4().to_string(),
            title: "Task with data".to_string(),
            column_id: src_col.id.clone(),
            creator: "alice".to_string(),
            created_at: now.clone(),
            updated_at: now,
            comments: vec![
                serde_json::json!({"user": "alice", "msg": "great task", "ts": "01-01 10:00"}),
            ],
            logs: vec![
                serde_json::json!({"user": "system", "msg": "created", "ts": "01-01 09:00"}),
            ],
            ..Task::default()
        };
        let task_id = task.id.clone();
        project.tasks.push(task);
        state.store.put_project(project).await.expect("put_project");

        let args = serde_json::json!({
            "task_id": task_id,
            "source_project_id": src.id,
            "target_project_id": dst.id
        });

        let result = execute_tool_pub(&state, "move_task_to_project", &args, "test")
            .await
            .expect("move_task_to_project should succeed");

        let new_task_id = result["task_id"].as_str().expect("task_id");
        let dst_updated = state.store.get_project(&dst.id).await.expect("get dst");
        let moved_task = dst_updated
            .tasks
            .iter()
            .find(|t| t.id == new_task_id)
            .expect("Task nicht gefunden");

        assert!(
            !moved_task.comments.is_empty(),
            "Kommentare sollen erhalten bleiben"
        );
        assert!(!moved_task.logs.is_empty(), "Logs sollen erhalten bleiben");
        assert_eq!(moved_task.creator, "alice", "Creator soll erhalten bleiben");
    }
}
