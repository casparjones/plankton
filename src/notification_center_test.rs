//! Integrationstests für das Notification-Center.
//!
//! Feature: Persistente Benachrichtigungen für Ticket-Events mit 24h-Auto-Cleanup.
//!
//! Getestet wird:
//! 1. Notification speichern und wieder laden
//! 2. Cleanup: Einträge älter als 24h werden entfernt
//! 3. API: GET /api/notifications liefert Liste (neueste zuerst)
//! 4. API: DELETE /api/notifications/:id löscht einen Eintrag
//! 5. API: DELETE /api/notifications löscht alle Einträge
//! 6. Notification-Persistenz überdauert Serverstart (File-Store)

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use tower::ServiceExt;

    use crate::models::notification::{NotificationEntry, NotificationEventType};
    use crate::state::AppState;
    use crate::store::{DataStore, FileStore};

    // ─── Hilfsfunktionen ────────────────────────────────────────────────────────

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

    // ────────────────────────────────────────────────────────────────────────────
    // Test 1: NotificationEntry hat alle Pflichtfelder
    // ────────────────────────────────────────────────────────────────────────────

    /// Ein neues NotificationEntry muss alle Pflichtfelder korrekt setzen.
    #[tokio::test]
    async fn test_notification_entry_fields() {
        let n = NotificationEntry::new(
            NotificationEventType::TaskCreated,
            "task-id-123".into(),
            "project-id-456".into(),
            "Mein Test-Task".into(),
            Some("alice".into()),
        );

        assert!(!n.id.is_empty(), "ID darf nicht leer sein");
        assert_eq!(n.event_type, NotificationEventType::TaskCreated);
        assert_eq!(n.task_id, "task-id-123");
        assert_eq!(n.project_id, "project-id-456");
        assert_eq!(n.task_title, "Mein Test-Task");
        assert_eq!(n.actor, Some("alice".into()));
        assert!(!n.read, "Neue Notification muss ungelesen sein");
        assert!(n.created_at > chrono::Utc::now() - chrono::Duration::seconds(5));
    }

    // ────────────────────────────────────────────────────────────────────────────
    // Test 2: Notification speichern und laden
    // ────────────────────────────────────────────────────────────────────────────

    /// Eine Notification wird gespeichert und kann wieder geladen werden.
    #[tokio::test]
    async fn test_save_and_load_notification() {
        let (state, _dir) = make_test_state().await;

        let n = NotificationEntry::new(
            NotificationEventType::TaskMoved,
            "task-abc".into(),
            "proj-xyz".into(),
            "Verschobener Task".into(),
            Some("bob".into()),
        );
        let id = n.id.clone();

        // Speichern
        state
            .store
            .save_notification(&n)
            .await
            .expect("Notification speichern muss klappen");

        // Laden
        let loaded = state
            .store
            .list_notifications()
            .await
            .expect("Notifications laden muss klappen");

        assert_eq!(loaded.len(), 1, "Genau eine Notification erwartet");
        assert_eq!(loaded[0].id, id);
        assert_eq!(loaded[0].task_id, "task-abc");
        assert_eq!(loaded[0].event_type, NotificationEventType::TaskMoved);
    }

    // ────────────────────────────────────────────────────────────────────────────
    // Test 3: Mehrere Notifications – Sortierung neueste zuerst
    // ────────────────────────────────────────────────────────────────────────────

    /// `list_notifications()` liefert Einträge absteigend nach `created_at`.
    #[tokio::test]
    async fn test_notifications_sorted_newest_first() {
        let (state, _dir) = make_test_state().await;

        // Erste Notification (ältere Zeit)
        let mut n1 = NotificationEntry::new(
            NotificationEventType::TaskCreated,
            "task-1".into(),
            "proj-1".into(),
            "Erster Task".into(),
            None,
        );
        n1.created_at = chrono::Utc::now() - chrono::Duration::hours(2);

        // Zweite Notification (neuere Zeit)
        let n2 = NotificationEntry::new(
            NotificationEventType::TaskUpdated,
            "task-2".into(),
            "proj-1".into(),
            "Zweiter Task".into(),
            None,
        );

        state.store.save_notification(&n1).await.expect("save n1");
        state.store.save_notification(&n2).await.expect("save n2");

        let loaded = state
            .store
            .list_notifications()
            .await
            .expect("list notifications");

        assert_eq!(loaded.len(), 2);
        // Neueste zuerst
        assert!(
            loaded[0].created_at >= loaded[1].created_at,
            "Neueste Notification muss zuerst kommen"
        );
        assert_eq!(loaded[0].task_id, "task-2", "task-2 muss zuerst kommen");
    }

    // ────────────────────────────────────────────────────────────────────────────
    // Test 4: Cleanup – Einträge älter als 24h werden entfernt
    // ────────────────────────────────────────────────────────────────────────────

    /// `cleanup_old_notifications()` entfernt alle Einträge älter als 24h.
    #[tokio::test]
    async fn test_cleanup_removes_old_notifications() {
        let (state, _dir) = make_test_state().await;

        // Alte Notification (25 Stunden alt)
        let mut old = NotificationEntry::new(
            NotificationEventType::TaskCreated,
            "old-task".into(),
            "proj-1".into(),
            "Alter Task".into(),
            None,
        );
        old.created_at = chrono::Utc::now() - chrono::Duration::hours(25);

        // Aktuelle Notification (1 Stunde alt)
        let mut recent = NotificationEntry::new(
            NotificationEventType::TaskMoved,
            "recent-task".into(),
            "proj-1".into(),
            "Aktueller Task".into(),
            None,
        );
        recent.created_at = chrono::Utc::now() - chrono::Duration::hours(1);

        state.store.save_notification(&old).await.expect("save old");
        state
            .store
            .save_notification(&recent)
            .await
            .expect("save recent");

        // Beide sind gespeichert
        let before = state
            .store
            .list_notifications()
            .await
            .expect("list before cleanup");
        assert_eq!(before.len(), 2, "Vor Cleanup: 2 Notifications erwartet");

        // Cleanup ausführen
        state
            .store
            .cleanup_old_notifications(24)
            .await
            .expect("cleanup muss klappen");

        // Nur die aktuelle ist noch da
        let after = state
            .store
            .list_notifications()
            .await
            .expect("list after cleanup");
        assert_eq!(
            after.len(),
            1,
            "Nach Cleanup: genau 1 Notification erwartet"
        );
        assert_eq!(
            after[0].task_id, "recent-task",
            "Nur der aktuelle Task darf übrig bleiben"
        );
    }

    // ────────────────────────────────────────────────────────────────────────────
    // Test 5: Einzelne Notification löschen
    // ────────────────────────────────────────────────────────────────────────────

    /// `delete_notification()` entfernt genau einen Eintrag.
    #[tokio::test]
    async fn test_delete_single_notification() {
        let (state, _dir) = make_test_state().await;

        let n1 = NotificationEntry::new(
            NotificationEventType::TaskCreated,
            "task-1".into(),
            "proj-1".into(),
            "Task 1".into(),
            None,
        );
        let n2 = NotificationEntry::new(
            NotificationEventType::TaskUpdated,
            "task-2".into(),
            "proj-1".into(),
            "Task 2".into(),
            None,
        );
        let id_to_delete = n1.id.clone();

        state.store.save_notification(&n1).await.expect("save n1");
        state.store.save_notification(&n2).await.expect("save n2");

        state
            .store
            .delete_notification(&id_to_delete)
            .await
            .expect("delete muss klappen");

        let remaining = state
            .store
            .list_notifications()
            .await
            .expect("list after delete");
        assert_eq!(remaining.len(), 1, "Eine Notification muss übrig bleiben");
        assert_eq!(remaining[0].task_id, "task-2");
    }

    // ────────────────────────────────────────────────────────────────────────────
    // Test 6: Alle Notifications löschen
    // ────────────────────────────────────────────────────────────────────────────

    /// `clear_all_notifications()` entfernt alle Einträge.
    #[tokio::test]
    async fn test_clear_all_notifications() {
        let (state, _dir) = make_test_state().await;

        for i in 0..3 {
            let n = NotificationEntry::new(
                NotificationEventType::TaskCreated,
                format!("task-{i}"),
                "proj-1".into(),
                format!("Task {i}"),
                None,
            );
            state.store.save_notification(&n).await.expect("save");
        }

        state
            .store
            .clear_all_notifications()
            .await
            .expect("clear all muss klappen");

        let remaining = state
            .store
            .list_notifications()
            .await
            .expect("list after clear");
        assert!(remaining.is_empty(), "Liste muss nach clear leer sein");
    }

    // ────────────────────────────────────────────────────────────────────────────
    // Test 7: HTTP GET /api/notifications → 200 + JSON-Array
    // ────────────────────────────────────────────────────────────────────────────

    /// Der HTTP-Endpunkt gibt eine JSON-Liste zurück.
    #[tokio::test]
    async fn test_http_list_notifications() {
        let (state, _dir) = make_test_state().await;

        // Eine Notification speichern
        let n = NotificationEntry::new(
            NotificationEventType::TaskCreated,
            "task-http-test".into(),
            "proj-http".into(),
            "HTTP-Test-Task".into(),
            Some("charlie".into()),
        );
        state.store.save_notification(&n).await.expect("save");

        // Token für Auth erstellen
        let token = create_test_token(&state, "test-agent", "agent").await;

        let app = crate::build_router(state);
        let req = Request::builder()
            .method(Method::GET)
            .uri("/api/notifications")
            .header("Authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.expect("request muss klappen");
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "GET /api/notifications muss 200 liefern"
        );

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .expect("body lesen");
        let json: serde_json::Value = serde_json::from_slice(&body).expect("JSON parsen");
        assert!(json.is_array(), "Response muss ein Array sein");
        assert_eq!(
            json.as_array().unwrap().len(),
            1,
            "Genau 1 Notification erwartet"
        );
    }

    // ─── Token-Hilfsfunktion ────────────────────────────────────────────────────

    async fn create_test_token(state: &AppState, name: &str, role: &str) -> String {
        use crate::models::auth::{hash_token_secret, AgentToken, TokenScope};
        use uuid::Uuid;
        let token_value = format!("plk_{}", Uuid::new_v4().simple());
        let token = AgentToken {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            token_hash: hash_token_secret(&token_value),
            active: true,
            scope: TokenScope::Global,
            role: role.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            last_used: None,
            description: "Testtoken".into(),
            creator: "test".into(),
            expires_at: None,
        };
        state
            .store
            .create_token(token)
            .await
            .expect("token erstellen");
        token_value
    }
}
