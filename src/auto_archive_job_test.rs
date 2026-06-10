//! Integrationstests für den stündlichen Background-Job: Auto-Archivierung & Auto-Delete.
//!
//! Testet die neue `run_maintenance_job`-Funktion in `project_service`:
//!
//! 1. `doneExpire: 0` → Task in Done-Spalte wird sofort archiviert
//! 2. `archiveDelete: 0` → Task in _archive-Spalte wird sofort gelöscht
//! 3. `doneExpire: -1` → kein Task aus Done verschoben
//! 4. `archiveDelete: -1` → kein archivierter Task gelöscht

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    use crate::models::{project::Task, ProjectDoc};
    use crate::services::project_service::{default_project, run_maintenance_job};
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
            last_maintenance_run: Arc::new(tokio::sync::RwLock::new(None)),
            started_at: chrono::Utc::now(),
            attachment_store: None,
        };
        (state, dir)
    }

    fn find_col(project: &ProjectDoc, title: &str) -> Option<String> {
        project
            .columns
            .iter()
            .find(|c| c.title == title)
            .map(|c| c.id.clone())
    }

    // -----------------------------------------------------------------------
    // Test 1: doneExpire: 0 → Task in Done-Spalte wird sofort archiviert
    // -----------------------------------------------------------------------

    /// Mit `doneExpire = 0` muss ein Task der sich gerade in Done befindet (column_entered_at = now)
    /// nach dem Job-Lauf in der _archive-Spalte liegen.
    #[tokio::test]
    async fn test_done_expire_zero_archives_task() {
        let (state, _dir) = make_test_state().await;

        let mut project = default_project("DoneExpireZeroTest".into());
        project.done_expire = Some(0);
        project.archive_delete = Some(-1); // Auto-Delete deaktiviert

        let done_id = find_col(&project, "Done").expect("Done column");
        let archive_id = find_col(&project, "_archive").expect("_archive column");

        let task_id = uuid::Uuid::new_v4().to_string();
        let task = Task {
            id: task_id.clone(),
            title: "Expire Task".into(),
            column_id: done_id.clone(),
            column_entered_at: Some(chrono::Utc::now()),
            ..Task::default()
        };
        project.tasks.push(task);
        let project = state.store.create_project(project).await.expect("create");

        run_maintenance_job(&state.store).await.expect("job");

        let updated = state.store.get_project(&project.id).await.expect("get");
        let task = updated
            .tasks
            .iter()
            .find(|t| t.id == task_id)
            .expect("task");

        assert_eq!(
            task.column_id, archive_id,
            "Task muss nach _archive verschoben worden sein (doneExpire=0)"
        );
    }

    // -----------------------------------------------------------------------
    // Test 2: archiveDelete: 0 → Task in _archive wird sofort gelöscht
    // -----------------------------------------------------------------------

    /// Mit `archiveDelete = 0` muss ein Task der sich in _archive befindet (column_entered_at = now)
    /// nach dem Job-Lauf gelöscht sein.
    #[tokio::test]
    async fn test_archive_delete_zero_deletes_task() {
        let (state, _dir) = make_test_state().await;

        let mut project = default_project("ArchiveDeleteZeroTest".into());
        project.done_expire = Some(-1); // Auto-Archivierung deaktiviert
        project.archive_delete = Some(0);

        let archive_id = find_col(&project, "_archive").expect("_archive column");

        let task_id = uuid::Uuid::new_v4().to_string();
        let task = Task {
            id: task_id.clone(),
            title: "Delete Me Task".into(),
            column_id: archive_id.clone(),
            column_entered_at: Some(chrono::Utc::now()),
            ..Task::default()
        };
        project.tasks.push(task);
        let project = state.store.create_project(project).await.expect("create");

        run_maintenance_job(&state.store).await.expect("job");

        let updated = state.store.get_project(&project.id).await.expect("get");
        let task_exists = updated.tasks.iter().any(|t| t.id == task_id);

        assert!(
            !task_exists,
            "Task muss gelöscht worden sein (archiveDelete=0)"
        );
    }

    // -----------------------------------------------------------------------
    // Test 3: doneExpire: -1 → kein Task aus Done verschoben
    // -----------------------------------------------------------------------

    /// Mit `doneExpire = -1` darf kein Task aus Done ins Archiv verschoben werden.
    #[tokio::test]
    async fn test_done_expire_disabled_does_not_archive() {
        let (state, _dir) = make_test_state().await;

        let mut project = default_project("DoneExpireDisabledTest".into());
        project.done_expire = Some(-1);
        project.archive_delete = Some(-1);

        let done_id = find_col(&project, "Done").expect("Done column");

        let task_id = uuid::Uuid::new_v4().to_string();
        let task = Task {
            id: task_id.clone(),
            title: "Stay In Done".into(),
            column_id: done_id.clone(),
            column_entered_at: Some(chrono::Utc::now() - chrono::Duration::days(9999)),
            ..Task::default()
        };
        project.tasks.push(task);
        let project = state.store.create_project(project).await.expect("create");

        run_maintenance_job(&state.store).await.expect("job");

        let updated = state.store.get_project(&project.id).await.expect("get");
        let task = updated
            .tasks
            .iter()
            .find(|t| t.id == task_id)
            .expect("task must still exist");

        assert_eq!(
            task.column_id, done_id,
            "Task darf nicht verschoben werden wenn doneExpire=-1"
        );
    }

    // -----------------------------------------------------------------------
    // Test 4: archiveDelete: -1 → kein archivierter Task gelöscht
    // -----------------------------------------------------------------------

    /// Mit `archiveDelete = -1` darf kein Task aus dem Archiv gelöscht werden.
    #[tokio::test]
    async fn test_archive_delete_disabled_does_not_delete() {
        let (state, _dir) = make_test_state().await;

        let mut project = default_project("ArchiveDeleteDisabledTest".into());
        project.done_expire = Some(-1);
        project.archive_delete = Some(-1);

        let archive_id = find_col(&project, "_archive").expect("_archive column");

        let task_id = uuid::Uuid::new_v4().to_string();
        let task = Task {
            id: task_id.clone(),
            title: "Stay In Archive".into(),
            column_id: archive_id.clone(),
            column_entered_at: Some(chrono::Utc::now() - chrono::Duration::days(9999)),
            ..Task::default()
        };
        project.tasks.push(task);
        let project = state.store.create_project(project).await.expect("create");

        run_maintenance_job(&state.store).await.expect("job");

        let updated = state.store.get_project(&project.id).await.expect("get");
        let task_exists = updated.tasks.iter().any(|t| t.id == task_id);

        assert!(
            task_exists,
            "Task darf nicht gelöscht werden wenn archiveDelete=-1"
        );
    }
}
