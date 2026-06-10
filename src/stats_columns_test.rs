//! Integrationstests für den Stats-Endpoint `GET /api/projects/:id/stats/columns`.
//!
//! Feature: `project_stats_columns`-Handler
//! - Gibt für jede Spalte `{column_id, title, task_count}` zurück.
//! - Zählt nur sichtbare (nicht-archivierte) Tasks.
//! - Performance: Endpoint-Logik ohne I/O-Overhead < 1ms.
//!
//! Diese Tests sind RED, solange die Implementierung fehlt.

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    use crate::controllers::project_controller::compute_column_stats;
    use crate::models::*;
    use crate::services::project_service::default_project;
    use crate::state::AppState;
    use crate::store::{DataStore, FileStore};

    /// Baut einen AppState mit temporärem File-Store für Tests.
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

    // -----------------------------------------------------------------------
    // Test 1: Leeres Projekt → alle Spalten mit count 0
    // -----------------------------------------------------------------------

    /// Bei einem Projekt ohne Tasks muss jede Spalte count=0 liefern.
    #[tokio::test]
    async fn test_stats_columns_empty_project() {
        let project = default_project("EmptyStatsTest".into());
        let col_count = project.columns.iter().filter(|c| !c.hidden).count();

        let stats = compute_column_stats(&project);

        assert_eq!(
            stats.len(),
            col_count,
            "Anzahl Einträge muss Spalten-Anzahl entsprechen"
        );
        for entry in &stats {
            assert_eq!(
                entry.task_count, 0,
                "Leeres Projekt: Spalte '{}' muss count=0 haben",
                entry.title
            );
        }
    }

    // -----------------------------------------------------------------------
    // Test 2: Tasks korrekt pro Spalte gezählt
    // -----------------------------------------------------------------------

    /// Tasks werden der richtigen Spalte zugeordnet.
    #[tokio::test]
    async fn test_stats_columns_counts_correctly() {
        let mut project = default_project("CountTest".into());

        // Erste sichtbare Spalte
        let col_id = project
            .columns
            .iter()
            .find(|c| !c.hidden)
            .map(|c| c.id.clone())
            .expect("must have at least one visible column");

        // 3 Tasks in erste Spalte einfügen
        for i in 0..3 {
            let task = Task {
                id: format!("task-{i}"),
                title: format!("Task {i}"),
                column_id: col_id.clone(),
                order: i,
                ..Default::default()
            };
            project.tasks.push(task);
        }

        let stats = compute_column_stats(&project);

        let col_stat = stats
            .iter()
            .find(|s| s.column_id == col_id)
            .expect("column must appear in stats");

        assert_eq!(col_stat.task_count, 3, "Spalte muss 3 Tasks zählen");

        // Alle anderen sichtbaren Spalten müssen 0 haben
        for entry in stats.iter().filter(|s| s.column_id != col_id) {
            assert_eq!(
                entry.task_count, 0,
                "Spalte '{}' muss 0 Tasks haben",
                entry.title
            );
        }
    }

    // -----------------------------------------------------------------------
    // Test 3: Archivierte (hidden) Spalten werden nicht mitgezählt
    // -----------------------------------------------------------------------

    /// Tasks in versteckten (archivierten) Spalten erscheinen nicht in stats.
    #[tokio::test]
    async fn test_stats_columns_excludes_hidden_columns() {
        let mut project = default_project("HiddenColTest".into());

        // Eine hidden-Spalte anlegen
        let hidden_col = Column {
            id: "hidden-col-1".to_string(),
            title: "_archive".to_string(),
            slug: "_archive".to_string(),
            order: 999,
            color: "#999999".to_string(),
            hidden: true,
            locked: false,
        };
        let hidden_col_id = hidden_col.id.clone();
        project.columns.push(hidden_col);

        // Task in die hidden-Spalte legen
        let task = Task {
            id: "archived-task-1".to_string(),
            title: "Archived Task".to_string(),
            column_id: hidden_col_id.clone(),
            order: 0,
            ..Default::default()
        };
        project.tasks.push(task);

        let stats = compute_column_stats(&project);

        // hidden-Spalte darf NICHT in stats auftauchen
        assert!(
            stats.iter().all(|s| s.column_id != hidden_col_id),
            "Versteckte Spalte darf nicht in stats erscheinen"
        );
    }

    // -----------------------------------------------------------------------
    // Test 4: Rückgabestruktur enthält column_id, title, task_count
    // -----------------------------------------------------------------------

    /// Jeder Stats-Eintrag muss die Felder `column_id`, `title` und `task_count` haben.
    #[tokio::test]
    async fn test_stats_columns_response_shape() {
        let project = default_project("ShapeTest".into());

        let stats = compute_column_stats(&project);

        assert!(
            !stats.is_empty(),
            "stats must not be empty for default project"
        );

        for entry in &stats {
            assert!(
                !entry.column_id.is_empty(),
                "column_id darf nicht leer sein"
            );
            assert!(!entry.title.is_empty(), "title darf nicht leer sein");
            // task_count ist u32, immer valide
        }
    }

    // -----------------------------------------------------------------------
    // Test 5: Endpoint über AppState abrufbar (Store-Integration)
    // -----------------------------------------------------------------------

    /// Der Handler liest das Projekt aus dem Store und gibt korrekte Stats zurück.
    #[tokio::test]
    async fn test_stats_columns_via_store() {
        let (state, _dir) = make_test_state().await;
        let mut project = default_project("StoreTest".into());
        let project_id = project.id.clone();

        // Task in erste sichtbare Spalte
        let col_id = project
            .columns
            .iter()
            .find(|c| !c.hidden)
            .map(|c| c.id.clone())
            .expect("must have visible column");

        let task = Task {
            id: "store-task-1".to_string(),
            title: "Store Task".to_string(),
            column_id: col_id.clone(),
            order: 0,
            ..Default::default()
        };
        project.tasks.push(task);

        state.store.create_project(project).await.expect("create");

        // Projekt aus Store laden und stats berechnen
        let loaded = state.store.get_project(&project_id).await.expect("get");
        let stats = compute_column_stats(&loaded);

        let col_stat = stats
            .iter()
            .find(|s| s.column_id == col_id)
            .expect("column must be in stats");

        assert_eq!(col_stat.task_count, 1, "Spalte muss 1 Task haben");
    }
}
