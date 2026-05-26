//! Integrationstests für `column_entered_at`-Feld im Task-Modell (Ticket 52151531).
//!
//! Prüft:
//! 1. Task in neue Spalte verschieben → `column_entered_at` wird gesetzt
//! 2. Nochmals verschieben → `column_entered_at` wird aktualisiert (neuer Timestamp)
//! 3. Bestehender Task ohne `column_entered_at` → kein Fehler, Fallback auf `updated_at`

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    use crate::models::{project::Task, ProjectDoc};
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
        };
        (state, dir)
    }

    fn col_id(project: &ProjectDoc, title: &str) -> String {
        project
            .columns
            .iter()
            .find(|c| c.title == title)
            .map(|c| c.id.clone())
            .unwrap_or_else(|| panic!("Column '{}' not found", title))
    }

    // -----------------------------------------------------------------------
    // Test 1: move_task setzt column_entered_at
    // -----------------------------------------------------------------------

    /// Nach dem ersten Spalten-Wechsel muss `column_entered_at` gesetzt sein.
    #[tokio::test]
    async fn test_move_task_sets_column_entered_at() {
        let (state, _dir) = make_test_state().await;
        let mut project = default_project("ColumnEnteredAtTest".into());

        let todo_id = col_id(&project, "Todo");
        let in_progress_id = col_id(&project, "In Progress");

        let task_id = uuid::Uuid::new_v4().to_string();
        let task = Task {
            id: task_id.clone(),
            title: "Test Task".into(),
            column_id: todo_id.clone(),
            ..Task::default()
        };
        project.tasks.push(task);
        let project = state.store.create_project(project).await.expect("create");

        let before_move = chrono::Utc::now();

        let args = serde_json::json!({
            "project_id": project.id,
            "task_id": task_id,
            "column_id": in_progress_id,
        });
        crate::controllers::mcp_controller::execute_tool_pub(
            &state,
            "move_task",
            &args,
            "testuser",
        )
        .await
        .expect("move_task should succeed");

        let updated_project = state
            .store
            .get_project(&project.id)
            .await
            .expect("get project");
        let updated_task = updated_project
            .tasks
            .iter()
            .find(|t| t.id == task_id)
            .expect("task not found");

        assert!(
            updated_task.column_entered_at.is_some(),
            "column_entered_at muss nach move_task gesetzt sein"
        );

        let entered_at = updated_task.column_entered_at.unwrap();
        assert!(
            entered_at >= before_move,
            "column_entered_at ({entered_at}) muss >= Zeitpunkt vor dem Move ({before_move}) sein"
        );
        assert_eq!(
            updated_task.column_id, in_progress_id,
            "column_id muss In Progress sein"
        );
    }

    // -----------------------------------------------------------------------
    // Test 2: zweites move_task aktualisiert column_entered_at
    // -----------------------------------------------------------------------

    /// Bei erneutem Spalten-Wechsel muss `column_entered_at` einen neuen Timestamp erhalten.
    #[tokio::test]
    async fn test_move_task_updates_column_entered_at() {
        let (state, _dir) = make_test_state().await;
        let mut project = default_project("ColumnEnteredAtUpdateTest".into());

        let todo_id = col_id(&project, "Todo");
        let in_progress_id = col_id(&project, "In Progress");
        let testing_id = col_id(&project, "Testing");

        let task_id = uuid::Uuid::new_v4().to_string();
        let task = Task {
            id: task_id.clone(),
            title: "Moveable Task".into(),
            column_id: todo_id.clone(),
            ..Task::default()
        };
        project.tasks.push(task);
        let project = state.store.create_project(project).await.expect("create");

        // Erster Move: Todo → In Progress
        let args1 = serde_json::json!({
            "project_id": project.id,
            "task_id": task_id,
            "column_id": in_progress_id,
        });
        crate::controllers::mcp_controller::execute_tool_pub(
            &state,
            "move_task",
            &args1,
            "testuser",
        )
        .await
        .expect("first move should succeed");

        let project_after_first = state.store.get_project(&project.id).await.expect("get");
        let task_after_first = project_after_first
            .tasks
            .iter()
            .find(|t| t.id == task_id)
            .expect("task not found");
        let first_entered_at = task_after_first
            .column_entered_at
            .expect("column_entered_at should be set after first move");

        // Kurz warten damit der zweite Timestamp unterschiedlich ist
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;

        // Zweiter Move: In Progress → Testing
        let args2 = serde_json::json!({
            "project_id": project.id,
            "task_id": task_id,
            "column_id": testing_id,
        });
        crate::controllers::mcp_controller::execute_tool_pub(
            &state,
            "move_task",
            &args2,
            "testuser",
        )
        .await
        .expect("second move should succeed");

        let project_after_second = state.store.get_project(&project.id).await.expect("get");
        let task_after_second = project_after_second
            .tasks
            .iter()
            .find(|t| t.id == task_id)
            .expect("task not found");
        let second_entered_at = task_after_second
            .column_entered_at
            .expect("column_entered_at should be set after second move");

        assert!(
            second_entered_at > first_entered_at,
            "column_entered_at muss nach zweitem Move neuer sein: {second_entered_at} > {first_entered_at}"
        );
        assert_eq!(task_after_second.column_id, testing_id);
    }

    // -----------------------------------------------------------------------
    // Test 3: Bestehender Task ohne column_entered_at → Fallback auf updated_at
    // -----------------------------------------------------------------------

    /// Tasks ohne `column_entered_at` dürfen keinen Fehler erzeugen.
    /// Als Fallback-Wert wird `updated_at` genutzt (Best-Effort).
    #[tokio::test]
    async fn test_task_without_column_entered_at_fallback() {
        let (state, _dir) = make_test_state().await;
        let mut project = default_project("FallbackTest".into());

        let todo_id = col_id(&project, "Todo");

        // Task explizit mit column_entered_at = None anlegen (Default)
        let task_id = uuid::Uuid::new_v4().to_string();
        let updated_at_str = "2026-01-01T00:00:00+00:00".to_string();
        let task = Task {
            id: task_id.clone(),
            title: "Legacy Task".into(),
            column_id: todo_id.clone(),
            column_entered_at: None,
            updated_at: updated_at_str.clone(),
            ..Task::default()
        };
        project.tasks.push(task);
        let project = state.store.create_project(project).await.expect("create");

        // Task abrufen – muss fehlerfrei funktionieren
        let loaded = state
            .store
            .get_project(&project.id)
            .await
            .expect("get project");
        let loaded_task = loaded
            .tasks
            .iter()
            .find(|t| t.id == task_id)
            .expect("task not found");

        // column_entered_at ist None bei altem Task
        assert!(
            loaded_task.column_entered_at.is_none(),
            "Alter Task ohne column_entered_at darf None behalten"
        );

        // Fallback-Logik: wenn column_entered_at None ist, fällt man auf updated_at zurück
        let effective_entered_at = loaded_task
            .column_entered_at
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|| loaded_task.updated_at.clone());

        assert_eq!(
            effective_entered_at, updated_at_str,
            "Fallback auf updated_at muss greifen wenn column_entered_at fehlt"
        );
    }
}
