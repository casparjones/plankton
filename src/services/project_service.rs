// Projekt-Hilfsfunktionen: Default-Projekt, SSE-Updates, Archivierung, Bootstrap.

use chrono::{Local, Utc};
use uuid::Uuid;

use crate::error::ApiError;
use crate::models::*;
use crate::state::AppState;
use crate::store::DataStore;
use crate::services::auth_service::hash_password;

/// SSE-Update an alle Listener eines Projekts senden und ggf. Git-Sync auslösen.
pub async fn publish_update(state: &AppState, project_id: &str) {
    let events = state.events.lock().await;
    if let Some(tx) = events.get(project_id) {
        let _ = tx.send(project_id.to_string());
    }
    drop(events);

    // Async Git-Sync auslösen (non-blocking)
    trigger_git_sync(state.clone(), project_id.to_string());
}

/// Löst einen asynchronen Git-Sync aus (fire-and-forget).
pub fn trigger_git_sync(state: AppState, project_id: String) {
    tokio::spawn(async move {
        // Projekt laden und prüfen ob Git aktiviert ist
        let project = match state.store.get_project(&project_id).await {
            Ok(p) => p,
            Err(_) => return,
        };
        match &project.git {
            Some(config) if config.enabled => {}
            _ => return,
        }
        if let Err(e) = crate::services::git_service::perform_git_sync(&state, &project_id).await {
            tracing::warn!("Auto-Git-Sync fehlgeschlagen für {project_id}: {e}");
        }
    });
}

/// Erstellt ein Projekt mit drei Default-Spalten (Todo, In Progress, Done) + versteckte _archive-Spalte.
pub fn default_project(title: String) -> ProjectDoc {
    ProjectDoc {
        id: Uuid::new_v4().to_string(),
        rev: None,
        title,
        columns: vec![
            Column {
                id: Uuid::new_v4().to_string(),
                title: "Todo".into(),
                slug: "TODO".into(),
                order: 0,
                color: "#90CAF9".into(),
                hidden: false,
                locked: true,
            },
            Column {
                id: Uuid::new_v4().to_string(),
                title: "In Progress".into(),
                slug: "IN_PROGRESS".into(),
                order: 1,
                color: "#FFCC80".into(),
                hidden: false,
                locked: false,
            },
            Column {
                id: Uuid::new_v4().to_string(),
                title: "Done".into(),
                slug: "DONE".into(),
                order: 2,
                color: "#A5D6A7".into(),
                hidden: false,
                locked: false,
            },
            Column {
                id: Uuid::new_v4().to_string(),
                title: "_archive".into(),
                slug: "_ARCHIVE".into(),
                order: 99,
                color: "#444".into(),
                hidden: true,
                locked: true,
            },
        ],
        users: vec![],
        tasks: vec![],
        git: None,
    }
}

/// Prüft alle Projekte und verschiebt Tasks, die ≥14 Tage in "Done" liegen,
/// in die versteckte "_archive"-Spalte.
pub async fn archive_old_tasks(store: &DataStore) -> Result<(), ApiError> {
    let projects = store.list_projects().await?;
    let cutoff = Utc::now() - chrono::Duration::days(14);

    for mut project in projects {
        let done_col_id = project.columns.iter()
            .find(|c| c.title == "Done")
            .map(|c| c.id.clone());
        let archive_col_id = project.columns.iter()
            .find(|c| c.title == "_archive")
            .map(|c| c.id.clone());

        let (done_id, archive_id) = match (done_col_id, archive_col_id) {
            (Some(d), Some(a)) => (d, a),
            _ => continue,
        };

        let mut changed = false;
        for task in &mut project.tasks {
            if task.column_id != done_id {
                continue;
            }
            let updated = chrono::DateTime::parse_from_rfc3339(&task.updated_at)
                .map(|dt| dt.with_timezone(&Utc));
            if let Ok(dt) = updated {
                if dt < cutoff {
                    task.previous_row = task.column_id.clone();
                    task.column_id = archive_id.clone();
                    task.updated_at = Utc::now().to_rfc3339();
                    task.logs.push(format!("{} auto-archived",
                        Local::now().format("%Y-%m-%d")));
                    changed = true;
                }
            }
        }

        if changed {
            store.put_project(project).await?;
            tracing::info!("Archivierung: Tasks in Projekt verschoben");
        }
    }
    Ok(())
}

/// Legt den Default-Admin an, falls noch kein Admin existiert.
pub async fn ensure_default_admin(store: &DataStore) -> Result<(), ApiError> {
    let users = store.list_users().await?;
    let has_admin = users.iter().any(|u| u.role == "admin");

    if !has_admin {
        let now = Utc::now().to_rfc3339();
        let admin = AuthUser {
            id: Uuid::new_v4().to_string(),
            username: "admin".into(),
            display_name: "Administrator".into(),
            password_hash: hash_password("admin")?,
            role: "admin".into(),
            created_at: now.clone(),
            updated_at: now,
            active: true,
        };
        store.create_user(admin).await?;
        println!(
            "  \x1b[1m\x1b[33mDefault admin created\x1b[0m (username: admin, password: admin)"
        );
    }
    Ok(())
}
