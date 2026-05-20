//! Integrationstests für Task-Blocking-Enforcement (Ticket fe83386e).
//!
//! Prüft:
//! 1. move_task nach "In Progress" schlägt fehl (400), wenn blocked_by-Task noch nicht in "Done"
//! 2. move_task nach "In Progress" erlaubt, wenn Blocker in "Done"
//! 3. move_task nach anderen Spalten (z.B. "Testing") ignoriert Blocking-Check
//! 4. Task ohne Blocker kann frei nach "In Progress" verschoben werden

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

    /// Hilfsfunktion: Gibt die Column-ID für einen Titel zurück.
    fn col_id(project: &ProjectDoc, title: &str) -> String {
        project
            .columns
            .iter()
            .find(|c| c.title == title)
            .map(|c| c.id.clone())
            .unwrap_or_else(|| panic!("Column '{}' not found", title))
    }

    // -----------------------------------------------------------------------
    // Test 1: Task mit offenem Blocker → move nach "In Progress" schlägt fehl
    // -----------------------------------------------------------------------

    /// Ein geblockter Task darf nicht nach "In Progress" verschoben werden,
    /// solange der Blocker-Task noch nicht in "Done" ist.
    #[tokio::test]
    async fn test_move_blocked_task_to_in_progress_fails() {
        let (state, _dir) = make_test_state().await;
        let mut project = default_project("BlockingTest".into());

        let todo_id = col_id(&project, "Todo");
        let in_progress_id = col_id(&project, "In Progress");

        // Blocker-Task: liegt in "Todo" (noch nicht Done)
        let blocker_id = uuid::Uuid::new_v4().to_string();
        let blocker = Task {
            id: blocker_id.clone(),
            title: "Blocker Task".into(),
            column_id: todo_id.clone(),
            ..Task::default()
        };

        // Geblockter Task: hat blocked_by = [blocker_id]
        let blocked_id = uuid::Uuid::new_v4().to_string();
        let blocked_task = Task {
            id: blocked_id.clone(),
            title: "Blocked Task".into(),
            column_id: todo_id.clone(),
            blocked_by: vec![blocker_id.clone()],
            ..Task::default()
        };

        project.tasks.push(blocker);
        project.tasks.push(blocked_task);
        let project = state.store.create_project(project).await.expect("create");

        // Versuch: geblockten Task nach "In Progress" verschieben → muss Fehler liefern
        let args = serde_json::json!({
            "project_id": project.id,
            "task_id": blocked_id,
            "column_id": in_progress_id,
        });
        let result = crate::controllers::mcp_controller::execute_tool_pub(
            &state,
            "move_task",
            &args,
            "testuser",
        )
        .await;

        assert!(
            result.is_err(),
            "move nach 'In Progress' mit offenem Blocker muss fehlschlagen"
        );
        let err_msg = format!("{:?}", result.err().unwrap());
        // Fehlermeldung muss Blocker-Titel oder ID enthalten
        assert!(
            err_msg.contains("Blocker Task") || err_msg.contains(&blocker_id),
            "Fehlermeldung muss Blocker nennen, war: {err_msg}"
        );
    }

    // -----------------------------------------------------------------------
    // Test 2: Blocker in "Done" → Move erlaubt
    // -----------------------------------------------------------------------

    /// Wenn der Blocker-Task in "Done" liegt, darf der geblockte Task nach "In Progress".
    #[tokio::test]
    async fn test_move_blocked_task_allowed_when_blocker_done() {
        let (state, _dir) = make_test_state().await;
        let mut project = default_project("BlockingAllowedTest".into());

        let todo_id = col_id(&project, "Todo");
        let in_progress_id = col_id(&project, "In Progress");
        let done_id = col_id(&project, "Done");

        // Blocker-Task: liegt in "Done"
        let blocker_id = uuid::Uuid::new_v4().to_string();
        let blocker = Task {
            id: blocker_id.clone(),
            title: "Done Blocker".into(),
            column_id: done_id.clone(),
            ..Task::default()
        };

        // Geblockter Task: hat blocked_by = [blocker_id], liegt in "Todo"
        let blocked_id = uuid::Uuid::new_v4().to_string();
        let blocked_task = Task {
            id: blocked_id.clone(),
            title: "Previously Blocked Task".into(),
            column_id: todo_id.clone(),
            blocked_by: vec![blocker_id.clone()],
            ..Task::default()
        };

        project.tasks.push(blocker);
        project.tasks.push(blocked_task);
        let project = state.store.create_project(project).await.expect("create");

        // Move nach "In Progress" muss erlaubt sein, weil Blocker Done ist
        let args = serde_json::json!({
            "project_id": project.id,
            "task_id": blocked_id,
            "column_id": in_progress_id,
        });
        let result = crate::controllers::mcp_controller::execute_tool_pub(
            &state,
            "move_task",
            &args,
            "testuser",
        )
        .await;

        assert!(
            result.is_ok(),
            "move nach 'In Progress' mit Blocker in Done muss erlaubt sein: {:?}",
            result.err()
        );
    }

    // -----------------------------------------------------------------------
    // Test 3: Move nach anderen Spalten ignoriert Blocker
    // -----------------------------------------------------------------------

    /// Blocker-Check gilt nur für "In Progress", nicht für andere Spalten.
    #[tokio::test]
    async fn test_move_blocked_task_to_other_column_allowed() {
        let (state, _dir) = make_test_state().await;
        let mut project = default_project("BlockingOtherColTest".into());

        let todo_id = col_id(&project, "Todo");
        let testing_id = col_id(&project, "Testing");

        // Blocker-Task: liegt in "Todo" (nicht Done)
        let blocker_id = uuid::Uuid::new_v4().to_string();
        let blocker = Task {
            id: blocker_id.clone(),
            title: "Open Blocker".into(),
            column_id: todo_id.clone(),
            ..Task::default()
        };

        // Geblockter Task
        let blocked_id = uuid::Uuid::new_v4().to_string();
        let blocked_task = Task {
            id: blocked_id.clone(),
            title: "Blocked Task Other Col".into(),
            column_id: todo_id.clone(),
            blocked_by: vec![blocker_id.clone()],
            ..Task::default()
        };

        project.tasks.push(blocker);
        project.tasks.push(blocked_task);
        let project = state.store.create_project(project).await.expect("create");

        // Move nach "Testing" (nicht "In Progress") → muss erlaubt sein
        let args = serde_json::json!({
            "project_id": project.id,
            "task_id": blocked_id,
            "column_id": testing_id,
        });
        let result = crate::controllers::mcp_controller::execute_tool_pub(
            &state,
            "move_task",
            &args,
            "testuser",
        )
        .await;

        assert!(
            result.is_ok(),
            "move nach anderen Spalten mit Blocker muss erlaubt sein: {:?}",
            result.err()
        );
    }

    // -----------------------------------------------------------------------
    // Test 4: Task ohne Blocker kann frei nach "In Progress"
    // -----------------------------------------------------------------------

    /// Kein blocked_by → keine Einschränkung.
    #[tokio::test]
    async fn test_move_unblocked_task_to_in_progress_succeeds() {
        let (state, _dir) = make_test_state().await;
        let mut project = default_project("UnblockedTest".into());

        let todo_id = col_id(&project, "Todo");
        let in_progress_id = col_id(&project, "In Progress");

        let task_id = uuid::Uuid::new_v4().to_string();
        let task = Task {
            id: task_id.clone(),
            title: "Free Task".into(),
            column_id: todo_id.clone(),
            blocked_by: vec![],
            ..Task::default()
        };

        project.tasks.push(task);
        let project = state.store.create_project(project).await.expect("create");

        let args = serde_json::json!({
            "project_id": project.id,
            "task_id": task_id,
            "column_id": in_progress_id,
        });
        let result = crate::controllers::mcp_controller::execute_tool_pub(
            &state,
            "move_task",
            &args,
            "testuser",
        )
        .await;

        assert!(
            result.is_ok(),
            "move ohne Blocker muss klappen: {:?}",
            result.err()
        );
    }

    // -----------------------------------------------------------------------
    // Test 5: Mehrere Blocker – mindestens einer offen → Fehler mit allen Blockern
    // -----------------------------------------------------------------------

    /// Bei mehreren Blockern, von denen einer offen ist, nennt die Fehlermeldung
    /// den offenen Blocker.
    #[tokio::test]
    async fn test_move_multiple_blockers_one_open_fails() {
        let (state, _dir) = make_test_state().await;
        let mut project = default_project("MultiBlockerTest".into());

        let todo_id = col_id(&project, "Todo");
        let in_progress_id = col_id(&project, "In Progress");
        let done_id = col_id(&project, "Done");

        // Blocker 1: Done
        let blocker1_id = uuid::Uuid::new_v4().to_string();
        let blocker1 = Task {
            id: blocker1_id.clone(),
            title: "Done Blocker".into(),
            column_id: done_id.clone(),
            ..Task::default()
        };

        // Blocker 2: noch offen (Todo)
        let blocker2_id = uuid::Uuid::new_v4().to_string();
        let blocker2 = Task {
            id: blocker2_id.clone(),
            title: "Open Blocker B".into(),
            column_id: todo_id.clone(),
            ..Task::default()
        };

        // Geblockt durch beide
        let blocked_id = uuid::Uuid::new_v4().to_string();
        let blocked_task = Task {
            id: blocked_id.clone(),
            title: "Multi Blocked Task".into(),
            column_id: todo_id.clone(),
            blocked_by: vec![blocker1_id.clone(), blocker2_id.clone()],
            ..Task::default()
        };

        project.tasks.push(blocker1);
        project.tasks.push(blocker2);
        project.tasks.push(blocked_task);
        let project = state.store.create_project(project).await.expect("create");

        let args = serde_json::json!({
            "project_id": project.id,
            "task_id": blocked_id,
            "column_id": in_progress_id,
        });
        let result = crate::controllers::mcp_controller::execute_tool_pub(
            &state,
            "move_task",
            &args,
            "testuser",
        )
        .await;

        assert!(
            result.is_err(),
            "move mit offenem Blocker muss fehlschlagen"
        );
        let err_msg = format!("{:?}", result.err().unwrap());
        // Nur der offene Blocker 2 soll genannt werden
        assert!(
            err_msg.contains("Open Blocker B") || err_msg.contains(&blocker2_id),
            "Fehlermeldung muss offenen Blocker nennen, war: {err_msg}"
        );
    }
}
