//! Integrationstests für das `type`-Feld am Project-Modell (kanban | list).
//!
//! Feature: Project-Datenmodell mit optionalem `type`-Feld
//! - Default/Fallback: `"kanban"` wenn das Feld fehlt oder leer ist
//! - `type` wird in GET-Responses ausgegeben
//! - `type` kann via MCP `update_project` gesetzt werden
//! - Kein Breaking Change: bestehende Projekte ohne `type` verhalten sich wie bisher
//!
//! Tests sind RED solange die Implementierung fehlt.

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::Mutex;

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
    // Test 1: Projekt ohne `type` → Default ist "kanban"
    // -----------------------------------------------------------------------

    /// Ein Projekt ohne explizites `type`-Feld muss beim Abrufen `"kanban"` liefern.
    #[tokio::test]
    async fn test_project_type_default_is_kanban() {
        let (state, _dir) = make_test_state().await;

        // Projekt ohne explizites type anlegen
        let project = default_project("TypeDefaultTest".into());
        let project_id = project.id.clone();
        state.store.create_project(project).await.expect("create");

        let loaded = state.store.get_project(&project_id).await.expect("get");
        assert_eq!(
            loaded.project_type(),
            "kanban",
            "Projekt ohne type muss 'kanban' zurückgeben"
        );
    }

    // -----------------------------------------------------------------------
    // Test 2: Projekt mit `type: "list"` → liefert "list"
    // -----------------------------------------------------------------------

    /// Ein Projekt mit `type = "list"` muss beim Abrufen `"list"` zurückgeben.
    #[tokio::test]
    async fn test_project_type_list_is_preserved() {
        let (state, _dir) = make_test_state().await;

        let mut project = default_project("TypeListTest".into());
        project.r#type = Some("list".to_string());
        let project_id = project.id.clone();
        state.store.create_project(project).await.expect("create");

        let loaded = state.store.get_project(&project_id).await.expect("get");
        assert_eq!(
            loaded.project_type(),
            "list",
            "Projekt mit type='list' muss 'list' zurückgeben"
        );
    }

    // -----------------------------------------------------------------------
    // Test 3: Update `type` von kanban → list → gespeichert und abrufbar
    // -----------------------------------------------------------------------

    /// Der `type` muss via Store-Update persistiert und danach korrekt abrufbar sein.
    #[tokio::test]
    async fn test_project_type_update_kanban_to_list() {
        let (state, _dir) = make_test_state().await;

        // Projekt ohne type anlegen (default = kanban)
        let project = default_project("TypeUpdateTest".into());
        let project_id = project.id.clone();
        state.store.create_project(project).await.expect("create");

        // Type auf "list" setzen
        let mut loaded = state.store.get_project(&project_id).await.expect("get");
        assert_eq!(loaded.project_type(), "kanban", "Vorher: kanban");

        loaded.r#type = Some("list".to_string());
        state.store.put_project(loaded).await.expect("put");

        // Erneut abrufen: muss "list" sein
        let updated = state
            .store
            .get_project(&project_id)
            .await
            .expect("get after update");
        assert_eq!(
            updated.project_type(),
            "list",
            "Nach Update muss type='list' gespeichert und abrufbar sein"
        );
    }

    // -----------------------------------------------------------------------
    // Test 4: `type` in serialisierter JSON-Ausgabe enthalten
    // -----------------------------------------------------------------------

    /// Der serialisierte JSON-Output des Projekts muss ein `"type"`-Feld enthalten.
    #[tokio::test]
    async fn test_project_type_in_json_output() {
        let (state, _dir) = make_test_state().await;

        // Projekt mit type="list"
        let mut project = default_project("JsonOutputTest".into());
        project.r#type = Some("list".to_string());
        let project_id = project.id.clone();
        state.store.create_project(project).await.expect("create");

        let loaded = state.store.get_project(&project_id).await.expect("get");
        let json = serde_json::to_value(&loaded).expect("serialize");

        assert_eq!(
            json["type"].as_str(),
            Some("list"),
            "JSON muss type='list' enthalten"
        );
    }

    // -----------------------------------------------------------------------
    // Test 5: Leeres type → Fallback auf "kanban"
    // -----------------------------------------------------------------------

    /// Ein Projekt mit `type = ""` (leer) soll via `project_type()` "kanban" liefern.
    #[tokio::test]
    async fn test_project_type_empty_string_fallback() {
        let mut project = default_project("EmptyTypeTest".into());
        // type explizit leer setzen
        project.r#type = Some(String::new());

        assert_eq!(
            project.project_type(),
            "kanban",
            "Leerer type-String muss auf 'kanban' fallen"
        );
    }

    // -----------------------------------------------------------------------
    // Test 6: default_project() hat kein explizites type (None → kanban)
    // -----------------------------------------------------------------------

    /// `default_project()` soll `type = None` liefern, Fallback ist "kanban".
    #[tokio::test]
    async fn test_default_project_type_is_none() {
        let project = default_project("DefaultTypeNoneTest".into());
        assert!(
            project.r#type.is_none(),
            "default_project sollte type=None haben"
        );
        assert_eq!(
            project.project_type(),
            "kanban",
            "Fallback muss 'kanban' sein"
        );
    }
}
