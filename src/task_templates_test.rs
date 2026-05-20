//! Integrationstests für Task-Templates (Ticket 6da71020).
//!
//! Feature: `create_task_from_template` MCP-Tool
//! - Liest Template aus `.plankton/templates/<name>.json` oder nutzt eingebettete Defaults
//! - Ersetzt Variablen `{{title}}` und `{{date}}` im Beschreibungstext
//! - Erstellt einen Task mit vorausgefüllten Feldern aus dem Template
//!
//! Diese Tests sind RED solange die Implementierung fehlt.

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    use crate::controllers::mcp_controller::execute_tool_pub;
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
        };
        (state, dir)
    }

    // -----------------------------------------------------------------------
    // Test 1: create_task_from_template mit eingebettetem "bug"-Template
    // -----------------------------------------------------------------------

    /// `create_task_from_template` mit Template "bug" erzeugt einen Task mit
    /// vorausgefüllter Beschreibung (Bug-Struktur) und Label "bug".
    #[tokio::test]
    async fn test_create_task_from_template_bug() {
        let (state, _dir) = make_test_state().await;
        let project = default_project("TemplateTest".into());
        let project_id = project.id.clone();
        state.store.create_project(project).await.expect("create");

        let args = serde_json::json!({
            "project_id": project_id,
            "template_name": "bug",
            "title": "Login schlägt fehl"
        });

        let result = execute_tool_pub(&state, "create_task_from_template", &args, "test-agent")
            .await
            .expect("create_task_from_template should succeed");

        // Task muss im Projekt gespeichert worden sein
        let task_id = result["id"].as_str().expect("result must have id");
        assert!(!task_id.is_empty(), "task_id must not be empty");

        // Titel muss aus dem Argument kommen
        assert_eq!(
            result["title"].as_str().unwrap(),
            "Login schlägt fehl",
            "title must match input"
        );

        // Label "bug" muss gesetzt sein (aus Template)
        let labels = result["labels"].as_array().expect("labels must be array");
        assert!(
            labels.iter().any(|l| l.as_str() == Some("bug")),
            "labels must contain 'bug'"
        );

        // Beschreibung muss Template-Inhalt haben (nicht leer)
        let desc = result["description"].as_str().unwrap_or("");
        assert!(
            desc.contains("## Problem"),
            "description must contain bug template structure, got: {desc}"
        );
    }

    // -----------------------------------------------------------------------
    // Test 2: Template-Variable {{title}} wird ersetzt
    // -----------------------------------------------------------------------

    /// {{title}} im Template wird durch den übergebenen Titel ersetzt.
    #[tokio::test]
    async fn test_template_variable_title_substitution() {
        let (state, _dir) = make_test_state().await;
        let project = default_project("VarSubstTest".into());
        let project_id = project.id.clone();
        state.store.create_project(project).await.expect("create");

        // Feature-Template hat {{title}} im Titelfeld des Templates
        let args = serde_json::json!({
            "project_id": project_id,
            "template_name": "feature",
            "title": "Dark Mode"
        });

        let result = execute_tool_pub(&state, "create_task_from_template", &args, "test-agent")
            .await
            .expect("create_task_from_template should succeed");

        let title = result["title"].as_str().unwrap_or("");
        // Kein unersetzter Platzhalter darf im Ergebnis stehen
        assert!(
            !title.contains("{{title}}"),
            "{{{{title}}}} placeholder must be replaced, got: {title}"
        );
        assert!(
            title.contains("Dark Mode"),
            "title must contain input value, got: {title}"
        );
    }

    // -----------------------------------------------------------------------
    // Test 3: Template-Variable {{date}} wird ersetzt
    // -----------------------------------------------------------------------

    /// {{date}} in der Template-Beschreibung wird durch das aktuelle Datum ersetzt.
    #[tokio::test]
    async fn test_template_variable_date_substitution() {
        let (state, _dir) = make_test_state().await;
        let project = default_project("DateSubstTest".into());
        let project_id = project.id.clone();
        state.store.create_project(project).await.expect("create");

        // Wir nutzen ein Template, das {{date}} in der Beschreibung haben sollte.
        // "chore" enthält {{date}} als Platzhalter laut Spec.
        let args = serde_json::json!({
            "project_id": project_id,
            "template_name": "chore",
            "title": "Abhängigkeiten aktualisieren"
        });

        let result = execute_tool_pub(&state, "create_task_from_template", &args, "test-agent")
            .await
            .expect("create_task_from_template should succeed");

        let desc = result["description"].as_str().unwrap_or("");
        // Kein unersetzter {{date}}-Platzhalter darf im Ergebnis stehen
        assert!(
            !desc.contains("{{date}}"),
            "{{{{date}}}} placeholder must be replaced in description, got: {desc}"
        );
    }

    // -----------------------------------------------------------------------
    // Test 4: Alle Standard-Templates sind verfügbar
    // -----------------------------------------------------------------------

    /// Alle 5 Standard-Templates (bug, feature, security, epic, chore) können
    /// genutzt werden ohne dass Dateien im Filesystem liegen müssen.
    #[tokio::test]
    async fn test_all_default_templates_available() {
        let (state, _dir) = make_test_state().await;
        let project = default_project("AllTemplatesTest".into());
        let project_id = project.id.clone();
        state.store.create_project(project).await.expect("create");

        for template_name in &["bug", "feature", "security", "epic", "chore"] {
            let args = serde_json::json!({
                "project_id": project_id,
                "template_name": template_name,
                "title": format!("Test {template_name}")
            });

            let result =
                execute_tool_pub(&state, "create_task_from_template", &args, "test-agent").await;

            assert!(
                result.is_ok(),
                "template '{template_name}' should be available as default, got: {:?}",
                result.err()
            );

            let result = result.unwrap();
            assert!(
                result["id"].as_str().is_some(),
                "template '{template_name}' result must have id"
            );
        }
    }

    // -----------------------------------------------------------------------
    // Test 5: Unbekanntes Template → Fehler
    // -----------------------------------------------------------------------

    /// Wenn weder eine lokale Datei noch ein Default-Template für den Namen
    /// existiert, muss ein Fehler zurückgegeben werden.
    #[tokio::test]
    async fn test_unknown_template_returns_error() {
        let (state, _dir) = make_test_state().await;
        let project = default_project("UnknownTemplateTest".into());
        let project_id = project.id.clone();
        state.store.create_project(project).await.expect("create");

        let args = serde_json::json!({
            "project_id": project_id,
            "template_name": "nonexistent_template_xyz",
            "title": "Irrelevant"
        });

        let result =
            execute_tool_pub(&state, "create_task_from_template", &args, "test-agent").await;

        assert!(
            result.is_err(),
            "unknown template must return an error, got: {:?}",
            result.ok()
        );
    }

    // -----------------------------------------------------------------------
    // Test 6: Lokale Datei in .plankton/templates/ überschreibt Default
    // -----------------------------------------------------------------------

    /// Eine lokale `.plankton/templates/bug.json`-Datei überschreibt das
    /// eingebettete Standard-Template.
    #[tokio::test]
    async fn test_local_template_file_overrides_default() {
        let (state, _dir) = make_test_state().await;
        let project = default_project("LocalTemplateTest".into());
        let project_id = project.id.clone();
        state.store.create_project(project).await.expect("create");

        // Lokales Template-Verzeichnis anlegen und Custom-Template schreiben
        let templates_dir = std::path::Path::new(".plankton/templates");
        std::fs::create_dir_all(templates_dir).expect("create .plankton/templates dir");
        let custom_template = serde_json::json!({
            "title": "CUSTOM BUG: {{title}}",
            "task_type": "task",
            "labels": ["bug", "custom"],
            "description": "## Custom Template\n\nDies ist ein lokales Template."
        });
        std::fs::write(
            templates_dir.join("bug.json"),
            serde_json::to_string_pretty(&custom_template).unwrap(),
        )
        .expect("write custom template");

        let args = serde_json::json!({
            "project_id": project_id,
            "template_name": "bug",
            "title": "Custom Bug"
        });

        let result = execute_tool_pub(&state, "create_task_from_template", &args, "test-agent")
            .await
            .expect("create_task_from_template with local template should succeed");

        // Beschreibung muss aus dem lokalen Template kommen
        let desc = result["description"].as_str().unwrap_or("");
        assert!(
            desc.contains("Custom Template"),
            "description must come from local template, got: {desc}"
        );

        // Label "custom" muss vorhanden sein (aus lokalem Template)
        let labels = result["labels"].as_array().expect("labels must be array");
        assert!(
            labels.iter().any(|l| l.as_str() == Some("custom")),
            "labels must contain 'custom' from local template"
        );

        // Aufräumen
        std::fs::remove_file(templates_dir.join("bug.json")).ok();
        std::fs::remove_dir(".plankton/templates").ok();
        std::fs::remove_dir(".plankton").ok();
    }

    // -----------------------------------------------------------------------
    // Test 7: create_task_from_template erscheint in tools/list
    // -----------------------------------------------------------------------

    /// Das Tool `create_task_from_template` muss in der Tool-Liste sichtbar sein.
    #[tokio::test]
    async fn test_create_task_from_template_in_tools_list() {
        // Wir prüfen, ob das Tool in `all_tools()` vorhanden ist, indem wir
        // es aufrufen – wenn das Tool unbekannt wäre, gäbe execute_tool_pub
        // "unknown tool" zurück, nicht einen anderen Fehler.
        let (state, _dir) = make_test_state().await;
        let project = default_project("ToolsListTest".into());
        let project_id = project.id.clone();
        state.store.create_project(project).await.expect("create");

        let args = serde_json::json!({
            "project_id": project_id,
            "template_name": "bug",
            "title": "Test"
        });

        let result =
            execute_tool_pub(&state, "create_task_from_template", &args, "test-agent").await;

        // Wenn das Tool existiert, darf der Fehler NICHT "unknown tool" sein
        if let Err(e) = &result {
            let msg = format!("{e:?}");
            assert!(
                !msg.contains("unknown tool"),
                "create_task_from_template must be a known tool, got: {msg}"
            );
        }
    }
}
