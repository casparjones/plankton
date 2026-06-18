// Projekt-Hilfsfunktionen: Default-Projekt, SSE-Updates, Archivierung, Bootstrap.

use chrono::Utc;
use uuid::Uuid;

use crate::error::ApiError;
use crate::models::*;
use crate::services::auth_service::hash_password;
use crate::state::AppState;
use crate::store::DataStore;

/// SSE-Update an alle Listener eines Projekts senden und ggf. Git-Sync auslösen.
/// Sendet ein generisches "project_update" Event (Full-Refetch).
pub async fn publish_update(state: &AppState, project_id: &str) {
    publish_event(state, project_id, "project_update", serde_json::json!({})).await;
}

/// Persistiert ein Ticket-Event als Notification im Notification-Center.
///
/// Fire-and-forget: Fehler werden geloggt aber nicht propagiert.
pub fn persist_notification(
    state: AppState,
    event_type: crate::models::NotificationEventType,
    task_id: String,
    task_title: String,
    project_id: String,
    actor: Option<String>,
) {
    tokio::spawn(async move {
        let n = crate::models::NotificationEntry::new(
            event_type, task_id, project_id, task_title, actor,
        );
        if let Err(e) = state.store.save_notification(&n).await {
            tracing::warn!("Notification-Persistierung fehlgeschlagen: {e}");
        }
    });
}

/// Granulares SSE-Event senden. Format: `{"event":"<type>","data":<payload>}`
/// Akzeptiert sowohl UUID als auch Slug als project_id.
pub async fn publish_event(
    state: &AppState,
    project_id: &str,
    event_type: &str,
    data: serde_json::Value,
) {
    let events = state.events.lock().await;
    // Versuche erst direkt, dann per aufgelöster ID (Slug → UUID).
    let found = if events.contains_key(project_id) {
        Some(project_id.to_string())
    } else {
        // Slug → UUID auflösen
        state
            .store
            .resolve_project_id(project_id)
            .await
            .ok()
            .filter(|real_id| events.contains_key(real_id.as_str()))
    };
    if let Some(ref key) = found {
        if let Some(tx) = events.get(key.as_str()) {
            let payload = serde_json::json!({
                "event": event_type,
                "data": data
            });
            let _ = tx.send(payload.to_string());
        }
    }
    drop(events);

    // Git-Sync deaktiviert
    // trigger_git_sync(state.clone(), project_id.to_string());
}

/// Löst einen asynchronen Git-Sync aus (fire-and-forget).
#[allow(dead_code)]
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

/// Erstellt ein Projekt mit vier Default-Spalten (Todo, In Progress, Testing, Done) + versteckte _archive-Spalte.
pub fn default_project(title: String) -> ProjectDoc {
    ProjectDoc {
        id: Uuid::new_v4().to_string(),
        rev: None,
        slug: crate::models::project_slugify(&title),
        owner: None,
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
                title: "Testing".into(),
                slug: "TESTING".into(),
                order: 2,
                color: "#CE93D8".into(),
                hidden: false,
                locked: false,
            },
            Column {
                id: Uuid::new_v4().to_string(),
                title: "Done".into(),
                slug: "DONE".into(),
                order: 3,
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
        webhook_url: None,
        order: 0,
        r#type: None,
        done_expire: None,
        archive_delete: None,
        pinned: None,
    }
}

/// Prüft alle Projekte und verschiebt Tasks, die ≥14 Tage in "Done" liegen,
/// in die versteckte "_archive"-Spalte.
///
/// Deprecated: Wird durch `run_maintenance_job` ersetzt, bleibt für Rückwärtskompatibilität.
#[allow(dead_code)]
pub async fn archive_old_tasks(store: &DataStore) -> Result<(), ApiError> {
    run_maintenance_job(store).await
}

/// Stündlicher Wartungs-Job: Auto-Archivierung und Auto-Delete.
///
/// Phase 1 – Done → Archiv:
/// Für jedes Projekt mit `doneExpire != -1`:
///   Tasks in Done-Spalten, deren `column_entered_at` älter als `doneExpire` Tage ist,
///   werden in die `_archive`-Spalte verschoben.
///
/// Phase 2 – Archiv → Löschen:
/// Für jedes Projekt mit `archiveDelete != -1`:
///   Tasks in `_archive`-Spalten, deren `column_entered_at` älter als `archiveDelete` Tage ist,
///   werden gelöscht.
///
/// Fehler pro Task werden isoliert – ein Fehler stoppt nicht den gesamten Job.
pub async fn run_maintenance_job(store: &DataStore) -> Result<(), ApiError> {
    let projects = store.list_projects().await?;
    let now = Utc::now();

    for mut project in projects {
        let project_id = project.id.clone();
        let done_expire = project.done_expire();
        let archive_delete = project.archive_delete();

        // Spalten-IDs ermitteln
        let archive_col_ids: Vec<String> = project
            .columns
            .iter()
            .filter(|c| c.title == "_archive")
            .map(|c| c.id.clone())
            .collect();

        // Done-Spalten: Titel enthält "Done" (case-insensitive), aber kein "_archive"
        let done_col_ids: Vec<String> = project
            .columns
            .iter()
            .filter(|c| c.title.to_lowercase().contains("done") && !c.title.starts_with('_'))
            .map(|c| c.id.clone())
            .collect();

        let mut changed = false;

        // --- Phase 1: Done → Archiv ---
        if done_expire != -1 && !archive_col_ids.is_empty() {
            let archive_id = archive_col_ids[0].clone();
            let cutoff = now - chrono::Duration::days(done_expire as i64);

            for task in &mut project.tasks {
                if !done_col_ids.contains(&task.column_id) {
                    continue;
                }
                // Effektiven Timestamp ermitteln: column_entered_at, sonst updated_at
                let effective_ts = task.column_entered_at.unwrap_or_else(|| {
                    chrono::DateTime::parse_from_rfc3339(&task.updated_at)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or(now)
                });
                if effective_ts <= cutoff {
                    tracing::info!(
                        task_id = %task.id,
                        project_id = %project_id,
                        action = "auto-archived",
                        "Maintenance-Job: Task archiviert"
                    );
                    task.previous_row = task.column_id.clone();
                    task.column_id = archive_id.clone();
                    task.updated_at = now.to_rfc3339();
                    task.column_entered_at = Some(now);
                    task.logs
                        .push(crate::models::log_entry("system", "auto-archived"));
                    changed = true;
                }
            }
        }

        // --- Phase 2: Archiv → Löschen ---
        if archive_delete != -1 && !archive_col_ids.is_empty() {
            let cutoff = now - chrono::Duration::days(archive_delete as i64);
            let mut to_delete: Vec<String> = Vec::new();

            for task in &project.tasks {
                if !archive_col_ids.contains(&task.column_id) {
                    continue;
                }
                let effective_ts = task.column_entered_at.unwrap_or_else(|| {
                    chrono::DateTime::parse_from_rfc3339(&task.updated_at)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or(now)
                });
                if effective_ts <= cutoff {
                    tracing::info!(
                        task_id = %task.id,
                        project_id = %project_id,
                        action = "auto-deleted",
                        "Maintenance-Job: Task gelöscht"
                    );
                    to_delete.push(task.id.clone());
                }
            }

            if !to_delete.is_empty() {
                // Relationen aufräumen
                for tid in &to_delete {
                    for task in &mut project.tasks {
                        task.blocks.retain(|id| id != tid);
                        task.blocked_by.retain(|id| id != tid);
                        task.subtask_ids.retain(|id| id != tid);
                        if &task.parent_id == tid {
                            task.parent_id.clear();
                        }
                    }
                }
                project.tasks.retain(|t| !to_delete.contains(&t.id));
                changed = true;
            }
        }

        if changed {
            if let Err(e) = store.put_project(project).await {
                tracing::error!(project_id = %project_id, error = %e, "Maintenance-Job: Fehler beim Speichern");
            }
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
