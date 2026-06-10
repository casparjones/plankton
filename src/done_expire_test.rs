//! Integrationstests für `doneExpire` + `archiveDelete` am Projekt-Modell.
//!
//! Feature: Zwei neue optionale Felder am ProjectDoc:
//! - `done_expire`: Tage bis Tasks aus Done ins Archiv verschoben werden. Default: 10. -1 = deaktiviert.
//! - `archive_delete`: Tage bis archivierte Tasks gelöscht werden. Default: 90. -1 = deaktiviert.
//!
//! Tests sind RED solange die Implementierung fehlt.

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    use crate::controllers::mcp_controller::execute_tool_pub;
    use crate::models::*;
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
            last_maintenance_run: Arc::new(tokio::sync::RwLock::new(None)),
            started_at: chrono::Utc::now(),
            attachment_store: None,
        };
        (state, dir)
    }

    // -----------------------------------------------------------------------
    // Test 1: Projekt ohne Felder → done_expire() == 10, archive_delete() == 90
    // -----------------------------------------------------------------------

    /// Ein Projekt ohne explizite Felder muss die Default-Werte liefern.
    #[tokio::test]
    async fn test_done_expire_default() {
        let project = default_project("ExpireDefaultTest".into());
        assert_eq!(
            project.done_expire(),
            10,
            "done_expire Default muss 10 sein"
        );
        assert_eq!(
            project.archive_delete(),
            90,
            "archive_delete Default muss 90 sein"
        );
    }

    // -----------------------------------------------------------------------
    // Test 2: Felder in JSON-Ausgabe immer enthalten
    // -----------------------------------------------------------------------

    /// Die serialisierte JSON-Ausgabe muss `doneExpire` und `archiveDelete` enthalten.
    #[tokio::test]
    async fn test_done_expire_in_json_output() {
        let project = default_project("JsonOutputExpireTest".into());
        let json = serde_json::to_value(&project).expect("serialize");

        assert_eq!(
            json["doneExpire"].as_i64(),
            Some(10),
            "JSON muss doneExpire=10 enthalten"
        );
        assert_eq!(
            json["archiveDelete"].as_i64(),
            Some(90),
            "JSON muss archiveDelete=90 enthalten"
        );
    }

    // -----------------------------------------------------------------------
    // Test 3: Persistenz – Werte speichern und lesen
    // -----------------------------------------------------------------------

    /// doneExpire und archiveDelete sollen persistiert und korrekt abrufbar sein.
    #[tokio::test]
    async fn test_done_expire_persist_and_reload() {
        let (state, _dir) = make_test_state().await;

        let mut project = default_project("PersistExpireTest".into());
        project.done_expire = Some(30);
        project.archive_delete = Some(180);
        let project_id = project.id.clone();
        state.store.create_project(project).await.expect("create");

        let loaded = state.store.get_project(&project_id).await.expect("get");
        assert_eq!(loaded.done_expire(), 30, "done_expire muss 30 sein");
        assert_eq!(loaded.archive_delete(), 180, "archive_delete muss 180 sein");
    }

    // -----------------------------------------------------------------------
    // Test 4: Update auf -1 (deaktiviert) via MCP update_project
    // -----------------------------------------------------------------------

    /// doneExpire = -1 und archiveDelete = -1 sollen gespeichert und abrufbar sein.
    #[tokio::test]
    async fn test_done_expire_update_to_minus_one() {
        let (state, _dir) = make_test_state().await;

        let project = default_project("DisabledExpireTest".into());
        let project_id = project.id.clone();
        state.store.create_project(project).await.expect("create");

        // Update via MCP
        let args = serde_json::json!({
            "project_id": project_id,
            "done_expire": -1,
            "archive_delete": -1,
        });
        execute_tool_pub(&state, "update_project", &args, "tester")
            .await
            .expect("update_project should succeed");

        let loaded = state.store.get_project(&project_id).await.expect("get");
        assert_eq!(
            loaded.done_expire(),
            -1,
            "done_expire=-1 muss gespeichert sein"
        );
        assert_eq!(
            loaded.archive_delete(),
            -1,
            "archive_delete=-1 muss gespeichert sein"
        );
    }

    // -----------------------------------------------------------------------
    // Test 5: Update auf validen Wert via MCP update_project
    // -----------------------------------------------------------------------

    /// doneExpire und archiveDelete sollen auf beliebige Werte setzbar sein.
    #[tokio::test]
    async fn test_done_expire_update_valid_value() {
        let (state, _dir) = make_test_state().await;

        let project = default_project("ValidExpireTest".into());
        let project_id = project.id.clone();
        state.store.create_project(project).await.expect("create");

        let args = serde_json::json!({
            "project_id": project_id,
            "done_expire": 7,
            "archive_delete": 365,
        });
        execute_tool_pub(&state, "update_project", &args, "tester")
            .await
            .expect("update_project should succeed");

        let loaded = state.store.get_project(&project_id).await.expect("get");
        assert_eq!(
            loaded.done_expire(),
            7,
            "done_expire=7 muss gespeichert sein"
        );
        assert_eq!(
            loaded.archive_delete(),
            365,
            "archive_delete=365 muss gespeichert sein"
        );
    }

    // -----------------------------------------------------------------------
    // Test 6: Altes Projekt ohne Felder in JSON → Default-Werte beim Deserialisieren
    // -----------------------------------------------------------------------

    /// Beim Deserialisieren eines JSON ohne die neuen Felder sollen Defaults greifen.
    #[tokio::test]
    async fn test_done_expire_missing_fields_deserialize_to_defaults() {
        // JSON ohne doneExpire / archiveDelete (simulates legacy project data)
        let json = serde_json::json!({
            "_id": "test-legacy-id",
            "title": "Legacy Project",
            "slug": "legacy-project",
            "columns": [],
            "users": [],
            "tasks": [],
            "order": 0,
        });
        let project: ProjectDoc = serde_json::from_value(json).expect("deserialize legacy project");

        assert_eq!(
            project.done_expire(),
            10,
            "Legacy-Projekt ohne doneExpire muss Default 10 liefern"
        );
        assert_eq!(
            project.archive_delete(),
            90,
            "Legacy-Projekt ohne archiveDelete muss Default 90 liefern"
        );
    }
}
