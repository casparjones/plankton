//! Integrationstests für Optimistic Locking via `_rev` (Ticket c75cecc2).
//!
//! Layer 1: Per-Projekt Write-Lock (Mutex) serialisiert alle Schreibzugriffe.
//! Layer 2: Optionaler `_rev`-Parameter in update_task/move_task/assign_task/delete_task.
//!
//! Diese Tests sind RED, solange die Implementierung fehlt.

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

        // Mindestens ein User für Auth
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
    // Layer 1: Per-Projekt Write-Lock
    // -----------------------------------------------------------------------

    /// AppState muss ein `write_locks`-Feld besitzen, das projektspezifische Mutexe enthält.
    #[tokio::test]
    async fn test_appstate_has_write_locks() {
        let (state, _dir) = make_test_state().await;
        // AppState muss write_locks anbieten – compile-time Prüfung.
        // Wir stellen sicher, dass ein Lock für ein Projekt geholt werden kann.
        let project = default_project("LockTest".into());
        let project_id = project.id.clone();
        state.store.create_project(project).await.expect("create");

        // Den Lock für das Projekt holen – must compile and not deadlock
        let _lock = state.get_project_write_lock(&project_id).await;
        // Wenn wir hier ankommen, ist Layer 1 implementiert.
    }

    // -----------------------------------------------------------------------
    // Layer 2: _rev Conflict-Checking via execute_tool
    // -----------------------------------------------------------------------

    /// update_task ohne `_rev` funktioniert weiterhin (Backward-Compat).
    #[tokio::test]
    async fn test_update_task_without_rev_succeeds() {
        let (state, _dir) = make_test_state().await;
        let mut project = default_project("RevTest".into());
        let task = Task {
            id: uuid::Uuid::new_v4().to_string(),
            title: "Original".into(),
            column_id: project.columns[0].id.clone(),
            ..Task::default()
        };
        project.tasks.push(task.clone());
        let project = state.store.create_project(project).await.expect("create");

        let args = serde_json::json!({
            "project_id": project.id,
            "task_id": task.id,
            "title": "Updated without rev"
        });

        let result = crate::controllers::mcp_controller::execute_tool_pub(
            &state,
            "update_task",
            &args,
            "testuser",
        )
        .await;
        assert!(
            result.is_ok(),
            "update ohne _rev muss klappen: {:?}",
            result.err()
        );
    }

    /// update_task mit korrekter `_rev` funktioniert.
    #[tokio::test]
    async fn test_update_task_with_correct_rev_succeeds() {
        let (state, _dir) = make_test_state().await;
        let mut project = default_project("RevTest2".into());
        let task = Task {
            id: uuid::Uuid::new_v4().to_string(),
            title: "Original".into(),
            column_id: project.columns[0].id.clone(),
            ..Task::default()
        };
        project.tasks.push(task.clone());
        let project = state.store.create_project(project).await.expect("create");

        // `_rev` des Projekts nach create_project ist "1"
        let args = serde_json::json!({
            "project_id": project.id,
            "task_id": task.id,
            "title": "Updated with correct rev",
            "_rev": "1"
        });

        let result = crate::controllers::mcp_controller::execute_tool_pub(
            &state,
            "update_task",
            &args,
            "testuser",
        )
        .await;
        assert!(
            result.is_ok(),
            "update mit korrekter _rev muss klappen: {:?}",
            result.err()
        );
    }

    /// update_task mit veralteter `_rev` → 409 Conflict.
    #[tokio::test]
    async fn test_update_task_with_stale_rev_returns_conflict() {
        let (state, _dir) = make_test_state().await;
        let mut project = default_project("RevConflict".into());
        let task = Task {
            id: uuid::Uuid::new_v4().to_string(),
            title: "Original".into(),
            column_id: project.columns[0].id.clone(),
            ..Task::default()
        };
        project.tasks.push(task.clone());
        let project = state.store.create_project(project).await.expect("create");

        // Erst ein normales Update durchführen – _rev wird auf "2" erhöht
        let first_update = serde_json::json!({
            "project_id": project.id,
            "task_id": task.id,
            "title": "First update"
        });
        crate::controllers::mcp_controller::execute_tool_pub(
            &state,
            "update_task",
            &first_update,
            "agent-a",
        )
        .await
        .expect("first update must succeed");

        // Zweites Update mit veralteter _rev "1" (aktuell wäre "2")
        let stale_args = serde_json::json!({
            "project_id": project.id,
            "task_id": task.id,
            "title": "Stale update",
            "_rev": "1"
        });

        let result = crate::controllers::mcp_controller::execute_tool_pub(
            &state,
            "update_task",
            &stale_args,
            "agent-b",
        )
        .await;

        match result {
            Err(crate::error::ApiError::Conflict(msg)) => {
                assert!(
                    msg.contains("current_rev") || msg.contains("conflict"),
                    "Conflict-Nachricht sollte 'current_rev' oder 'conflict' enthalten: {msg}"
                );
            }
            Err(e) => panic!("Erwartete Conflict-Error, bekam: {:?}", e),
            Ok(_) => panic!("Stale _rev hätte Conflict liefern sollen"),
        }
    }

    /// Zwei parallele Updates auf demselben Task: einer bekommt 200, einer 409.
    #[tokio::test]
    async fn test_concurrent_updates_one_wins_one_conflicts() {
        let (state, _dir) = make_test_state().await;
        let mut project = default_project("Concurrent".into());
        let task = Task {
            id: uuid::Uuid::new_v4().to_string(),
            title: "Shared".into(),
            column_id: project.columns[0].id.clone(),
            ..Task::default()
        };
        project.tasks.push(task.clone());
        let project = state.store.create_project(project).await.expect("create");

        let project_id = project.id.clone();
        let task_id = task.id.clone();

        // Beide Agents lesen _rev "1" und versuchen gleichzeitig zu schreiben
        let state_a = state.clone();
        let state_b = state.clone();
        let pid_a = project_id.clone();
        let pid_b = project_id.clone();
        let tid_a = task_id.clone();
        let tid_b = task_id.clone();

        let handle_a = tokio::spawn(async move {
            let args = serde_json::json!({
                "project_id": pid_a,
                "task_id": tid_a,
                "title": "Agent A wins",
                "_rev": "1"
            });
            crate::controllers::mcp_controller::execute_tool_pub(
                &state_a,
                "update_task",
                &args,
                "agent-a",
            )
            .await
        });

        let handle_b = tokio::spawn(async move {
            let args = serde_json::json!({
                "project_id": pid_b,
                "task_id": tid_b,
                "title": "Agent B wins",
                "_rev": "1"
            });
            crate::controllers::mcp_controller::execute_tool_pub(
                &state_b,
                "update_task",
                &args,
                "agent-b",
            )
            .await
        });

        let result_a = handle_a.await.expect("spawn a");
        let result_b = handle_b.await.expect("spawn b");

        let successes = [result_a.is_ok(), result_b.is_ok()]
            .iter()
            .filter(|&&ok| ok)
            .count();
        let conflicts = [result_a.is_err(), result_b.is_err()]
            .iter()
            .filter(|&&err| err)
            .count();

        assert_eq!(successes, 1, "Genau ein Update soll durchkommen");
        assert_eq!(conflicts, 1, "Genau ein Update soll 409 bekommen");
    }

    /// move_task mit veralteter _rev → Conflict.
    #[tokio::test]
    async fn test_move_task_with_stale_rev_returns_conflict() {
        let (state, _dir) = make_test_state().await;
        let mut project = default_project("MoveConflict".into());
        let task = Task {
            id: uuid::Uuid::new_v4().to_string(),
            title: "MoveMe".into(),
            column_id: project.columns[0].id.clone(),
            ..Task::default()
        };
        let col2_id = project.columns[1].id.clone();
        project.tasks.push(task.clone());
        let project = state.store.create_project(project).await.expect("create");

        // Normales Update – _rev wird auf "2"
        let first_update = serde_json::json!({
            "project_id": project.id,
            "task_id": task.id,
            "title": "touched"
        });
        crate::controllers::mcp_controller::execute_tool_pub(
            &state,
            "update_task",
            &first_update,
            "agent-a",
        )
        .await
        .expect("first update");

        // move_task mit _rev "1" (veraltet)
        let move_args = serde_json::json!({
            "project_id": project.id,
            "task_id": task.id,
            "column_id": col2_id,
            "_rev": "1"
        });
        let result = crate::controllers::mcp_controller::execute_tool_pub(
            &state,
            "move_task",
            &move_args,
            "agent-b",
        )
        .await;

        assert!(
            result.is_err(),
            "move_task mit staler _rev muss Conflict liefern"
        );
        match result.unwrap_err() {
            crate::error::ApiError::Conflict(_) => {}
            e => panic!("Falscher Fehlertyp: {:?}", e),
        }
    }

    /// delete_task mit veralteter _rev → Conflict.
    #[tokio::test]
    async fn test_delete_task_with_stale_rev_returns_conflict() {
        let (state, _dir) = make_test_state().await;
        let mut project = default_project("DeleteConflict".into());
        let task = Task {
            id: uuid::Uuid::new_v4().to_string(),
            title: "DeleteMe".into(),
            column_id: project.columns[0].id.clone(),
            ..Task::default()
        };
        project.tasks.push(task.clone());
        let project = state.store.create_project(project).await.expect("create");

        // Normales Update → _rev "2"
        let touch = serde_json::json!({
            "project_id": project.id,
            "task_id": task.id,
            "title": "touched"
        });
        crate::controllers::mcp_controller::execute_tool_pub(
            &state,
            "update_task",
            &touch,
            "agent-a",
        )
        .await
        .expect("touch");

        // delete mit veralteter _rev
        let args = serde_json::json!({
            "project_id": project.id,
            "task_id": task.id,
            "_rev": "1"
        });
        let result = crate::controllers::mcp_controller::execute_tool_pub(
            &state,
            "delete_task",
            &args,
            "agent-b",
        )
        .await;

        assert!(
            result.is_err(),
            "delete_task mit staler _rev muss Conflict liefern"
        );
        match result.unwrap_err() {
            crate::error::ApiError::Conflict(_) => {}
            e => panic!("Falscher Fehlertyp: {:?}", e),
        }
    }

    /// assign_task mit veralteter _rev → Conflict.
    #[tokio::test]
    async fn test_assign_task_with_stale_rev_returns_conflict() {
        let (state, _dir) = make_test_state().await;
        let mut project = default_project("AssignConflict".into());
        let task = Task {
            id: uuid::Uuid::new_v4().to_string(),
            title: "AssignMe".into(),
            column_id: project.columns[0].id.clone(),
            ..Task::default()
        };
        project.tasks.push(task.clone());
        let project = state.store.create_project(project).await.expect("create");

        // Touch → _rev "2"
        let touch = serde_json::json!({
            "project_id": project.id,
            "task_id": task.id,
            "title": "touched"
        });
        crate::controllers::mcp_controller::execute_tool_pub(
            &state,
            "update_task",
            &touch,
            "agent-a",
        )
        .await
        .expect("touch");

        let args = serde_json::json!({
            "project_id": project.id,
            "task_id": task.id,
            "worker": "bob",
            "_rev": "1"
        });
        let result = crate::controllers::mcp_controller::execute_tool_pub(
            &state,
            "assign_task",
            &args,
            "agent-b",
        )
        .await;

        assert!(
            result.is_err(),
            "assign_task mit staler _rev muss Conflict liefern"
        );
        match result.unwrap_err() {
            crate::error::ApiError::Conflict(_) => {}
            e => panic!("Falscher Fehlertyp: {:?}", e),
        }
    }

    /// add_comment bleibt rev-frei (append-only, kein Conflict).
    #[tokio::test]
    async fn test_add_comment_is_rev_free() {
        let (state, _dir) = make_test_state().await;
        let mut project = default_project("CommentRevFree".into());
        let task = Task {
            id: uuid::Uuid::new_v4().to_string(),
            title: "CommentTarget".into(),
            column_id: project.columns[0].id.clone(),
            ..Task::default()
        };
        project.tasks.push(task.clone());
        let project = state.store.create_project(project).await.expect("create");

        // Touch → _rev "2"
        let touch = serde_json::json!({
            "project_id": project.id,
            "task_id": task.id,
            "title": "touched"
        });
        crate::controllers::mcp_controller::execute_tool_pub(
            &state,
            "update_task",
            &touch,
            "agent-a",
        )
        .await
        .expect("touch");

        // add_comment mit alter _rev muss trotzdem klappen (rev-frei)
        let comment_args = serde_json::json!({
            "project_id": project.id,
            "task_id": task.id,
            "text": "Kommentar trotz staler _rev"
        });
        let result = crate::controllers::mcp_controller::execute_tool_pub(
            &state,
            "add_comment",
            &comment_args,
            "agent-b",
        )
        .await;
        assert!(
            result.is_ok(),
            "add_comment darf keine _rev-Prüfung haben: {:?}",
            result.err()
        );
    }

    /// Conflict-Response enthält `current_rev` im Fehler.
    #[tokio::test]
    async fn test_conflict_response_contains_current_rev() {
        let (state, _dir) = make_test_state().await;
        let mut project = default_project("CurrentRevTest".into());
        let task = Task {
            id: uuid::Uuid::new_v4().to_string(),
            title: "CurrentRevTask".into(),
            column_id: project.columns[0].id.clone(),
            ..Task::default()
        };
        project.tasks.push(task.clone());
        let project = state.store.create_project(project).await.expect("create");

        // Touch → _rev "2"
        let touch = serde_json::json!({
            "project_id": project.id,
            "task_id": task.id,
            "title": "touched"
        });
        crate::controllers::mcp_controller::execute_tool_pub(
            &state,
            "update_task",
            &touch,
            "agent-a",
        )
        .await
        .expect("touch");

        // Stale update
        let args = serde_json::json!({
            "project_id": project.id,
            "task_id": task.id,
            "title": "stale",
            "_rev": "1"
        });
        let result = crate::controllers::mcp_controller::execute_tool_pub(
            &state,
            "update_task",
            &args,
            "agent-b",
        )
        .await;

        match result {
            Err(crate::error::ApiError::Conflict(msg)) => {
                assert!(
                    msg.contains("current_rev") || msg.contains("2"),
                    "Conflict-Nachricht soll aktuelle Rev enthalten: {msg}"
                );
            }
            Err(e) => panic!("Erwartete Conflict, bekam: {:?}", e),
            Ok(_) => panic!("Stale Rev muss Conflict liefern"),
        }
    }
}
