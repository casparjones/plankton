//! Tester-Integrationstests für `run_maintenance_job` mit konkreten Zeitdifferenzen.
//!
//! Szenarien:
//! 1. Task in Done, 11 Tage alt + doneExpire:10 → wird in _archive verschoben
//! 2. Task in Done, 5 Tage alt  + doneExpire:10 → bleibt in Done
//! 3. doneExpire:-1              → kein Task wird verschoben (egal wie alt)
//! 4. Archivierter Task älter als archiveDelete Tage → wird gelöscht
//! 5. archiveDelete:-1           → kein Task wird gelöscht (egal wie alt)

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
    // Szenario 1: Task 11 Tage alt, doneExpire:10 → muss archiviert werden
    // -----------------------------------------------------------------------

    /// Ein Task, der seit 11 Tagen in Done liegt (doneExpire=10),
    /// muss nach dem Job-Lauf in der _archive-Spalte liegen.
    #[tokio::test]
    async fn test_task_11_days_old_with_done_expire_10_gets_archived() {
        let (state, _dir) = make_test_state().await;

        let mut project = default_project("Expire11Days".into());
        project.done_expire = Some(10);
        project.archive_delete = Some(-1); // Auto-Delete deaktiviert

        let done_id = find_col(&project, "Done").expect("Done column");
        let archive_id = find_col(&project, "_archive").expect("_archive column");

        let task_id = uuid::Uuid::new_v4().to_string();
        let task = Task {
            id: task_id.clone(),
            title: "Old Task 11 days".into(),
            column_id: done_id.clone(),
            column_entered_at: Some(chrono::Utc::now() - chrono::Duration::days(11)),
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
            .expect("task must still exist (jetzt in _archive)");

        assert_eq!(
            task.column_id, archive_id,
            "Task (11 Tage alt, doneExpire=10) muss in _archive verschoben worden sein"
        );
    }

    // -----------------------------------------------------------------------
    // Szenario 2: Task 5 Tage alt, doneExpire:10 → bleibt in Done
    // -----------------------------------------------------------------------

    /// Ein Task, der erst 5 Tage in Done liegt (doneExpire=10),
    /// darf nach dem Job-Lauf NICHT archiviert werden.
    #[tokio::test]
    async fn test_task_5_days_old_with_done_expire_10_stays_in_done() {
        let (state, _dir) = make_test_state().await;

        let mut project = default_project("NoBefore10Days".into());
        project.done_expire = Some(10);
        project.archive_delete = Some(-1);

        let done_id = find_col(&project, "Done").expect("Done column");

        let task_id = uuid::Uuid::new_v4().to_string();
        let task = Task {
            id: task_id.clone(),
            title: "Recent Task 5 days".into(),
            column_id: done_id.clone(),
            column_entered_at: Some(chrono::Utc::now() - chrono::Duration::days(5)),
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
            "Task (5 Tage alt, doneExpire=10) darf NICHT archiviert werden"
        );
    }

    // -----------------------------------------------------------------------
    // Szenario 3: doneExpire:-1 → kein Task wird verschoben
    // -----------------------------------------------------------------------

    /// Mit doneExpire=-1 darf kein Task archiviert werden, egal wie alt er ist.
    #[tokio::test]
    async fn test_done_expire_minus_one_never_archives() {
        let (state, _dir) = make_test_state().await;

        let mut project = default_project("DoneExpireDisabled".into());
        project.done_expire = Some(-1);
        project.archive_delete = Some(-1);

        let done_id = find_col(&project, "Done").expect("Done column");

        let task_id = uuid::Uuid::new_v4().to_string();
        let task = Task {
            id: task_id.clone(),
            title: "Very Old Task – should stay".into(),
            column_id: done_id.clone(),
            // Extrem alt: 9999 Tage
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
            "doneExpire=-1: Task darf nie aus Done verschoben werden"
        );
    }

    // -----------------------------------------------------------------------
    // Szenario 4: Archivierter Task älter als archiveDelete → wird gelöscht
    // -----------------------------------------------------------------------

    /// Ein Task in _archive mit column_entered_at älter als archiveDelete Tage
    /// muss nach dem Job-Lauf gelöscht sein.
    #[tokio::test]
    async fn test_archived_task_older_than_archive_delete_gets_deleted() {
        let (state, _dir) = make_test_state().await;

        let mut project = default_project("ArchiveDeleteOld".into());
        project.done_expire = Some(-1); // Auto-Archivierung deaktiviert
        project.archive_delete = Some(30);

        let archive_id = find_col(&project, "_archive").expect("_archive column");

        let task_id = uuid::Uuid::new_v4().to_string();
        let task = Task {
            id: task_id.clone(),
            title: "Old Archive Task".into(),
            column_id: archive_id.clone(),
            // 31 Tage alt → überschreitet archiveDelete:30
            column_entered_at: Some(chrono::Utc::now() - chrono::Duration::days(31)),
            ..Task::default()
        };
        project.tasks.push(task);
        let project = state.store.create_project(project).await.expect("create");

        run_maintenance_job(&state.store).await.expect("job");

        let updated = state.store.get_project(&project.id).await.expect("get");
        let task_exists = updated.tasks.iter().any(|t| t.id == task_id);

        assert!(
            !task_exists,
            "Archivierter Task (31 Tage alt, archiveDelete=30) muss gelöscht worden sein"
        );
    }

    // -----------------------------------------------------------------------
    // Szenario 5: archiveDelete:-1 → kein Task wird gelöscht
    // -----------------------------------------------------------------------

    /// Mit archiveDelete=-1 darf kein archivierter Task gelöscht werden,
    /// egal wie alt er ist.
    #[tokio::test]
    async fn test_archive_delete_minus_one_never_deletes() {
        let (state, _dir) = make_test_state().await;

        let mut project = default_project("ArchiveDeleteDisabled".into());
        project.done_expire = Some(-1);
        project.archive_delete = Some(-1);

        let archive_id = find_col(&project, "_archive").expect("_archive column");

        let task_id = uuid::Uuid::new_v4().to_string();
        let task = Task {
            id: task_id.clone(),
            title: "Permanent Archive Task".into(),
            column_id: archive_id.clone(),
            // Extrem alt
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
            "archiveDelete=-1: Archivierter Task darf nie gelöscht werden"
        );
    }
}
