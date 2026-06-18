// Datenmodell für das Notification-Center.
//
// Jeder Eintrag repräsentiert ein Ticket-Event (task_created, task_moved, etc.)
// das persistent gespeichert und über die API abgerufen werden kann.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Typ des Ticket-Events, das die Notification ausgelöst hat.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::enum_variant_names)]
pub enum NotificationEventType {
    TaskCreated,
    TaskMoved,
    TaskUpdated,
    TaskCommented,
    TaskDeleted,
}

/// Eine einzelne persistierte Benachrichtigung.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NotificationEntry {
    /// Eindeutige ID (UUID v4).
    pub id: String,
    /// Typ des Events.
    pub event_type: NotificationEventType,
    /// ID des betroffenen Tasks.
    pub task_id: String,
    /// Titel des betroffenen Tasks (zum Anzeigen, ohne Extra-API-Call).
    pub task_title: String,
    /// ID des betroffenen Projekts.
    pub project_id: String,
    /// Wer die Aktion ausgelöst hat (optional).
    pub actor: Option<String>,
    /// Wurde die Notification bereits gelesen?
    pub read: bool,
    /// Zeitstempel der Erstellung.
    pub created_at: DateTime<Utc>,
}

impl NotificationEntry {
    /// Neue Notification erstellen mit aktuellem Timestamp.
    pub fn new(
        event_type: NotificationEventType,
        task_id: String,
        project_id: String,
        task_title: String,
        actor: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            event_type,
            task_id,
            task_title,
            project_id,
            actor,
            read: false,
            created_at: Utc::now(),
        }
    }
}
