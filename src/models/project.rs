// Datenmodelle für Projekte, Spalten, Nutzer und Aufgaben.

use serde::{Deserialize, Serialize};

/// Repräsentiert ein vollständiges Kanban-Projekt als flaches Dokument.
/// Sowohl CouchDB-Felder (`_id`, `_rev`) als auch die eigentlichen Daten
/// (Spalten, Nutzer, Aufgaben) sind enthalten.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectDoc {
    #[serde(rename = "_id")]
    pub id: String,
    /// Revisions-Token – wird von CouchDB benötigt und im FileStore simuliert.
    #[serde(rename = "_rev", skip_serializing_if = "Option::is_none")]
    pub rev: Option<String>,
    pub title: String,
    /// URL-freundlicher Slug (auto-generiert aus Titel, z.B. "mein-projekt").
    #[serde(default)]
    pub slug: String,
    pub columns: Vec<Column>,
    pub users: Vec<User>,
    pub tasks: Vec<Task>,
    /// Optionale Git-Repository-Konfiguration für automatische Synchronisation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git: Option<GitConfig>,
}

/// Git-Repository-Konfiguration für ein Projekt.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitConfig {
    /// Repository-URL (HTTPS oder SSH).
    pub repo_url: String,
    /// Branch-Name (Standard: "main").
    #[serde(default = "default_branch")]
    pub branch: String,
    /// Pfad innerhalb des Repos, in dem die Projektdatei gespeichert wird.
    #[serde(default = "default_path")]
    pub path: String,
    /// Ob die Git-Synchronisation aktiv ist.
    #[serde(default)]
    pub enabled: bool,
    /// Zeitstempel des letzten erfolgreichen Push.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_push: Option<String>,
    /// Letzte Fehlermeldung bei Push-Fehler.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

fn default_branch() -> String { "main".to_string() }
fn default_path() -> String { "plankton.json".to_string() }

/// Eine Spalte im Kanban-Board.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Column {
    pub id: String,
    pub title: String,
    /// Normalisierter Slug, z.B. "TODO", "IN_PROGRESS".
    #[serde(default)]
    pub slug: String,
    /// Reihenfolge der Spalte (aufsteigend).
    pub order: i32,
    /// Hex-Farbcode, z.B. "#90CAF9".
    pub color: String,
    /// Versteckte Spalten (z.B. _archive) werden im Frontend nicht angezeigt.
    #[serde(default)]
    pub hidden: bool,
    /// Geschützte Spalten können nicht gelöscht werden.
    #[serde(default)]
    pub locked: bool,
}

/// Generiert einen normalisierten Slug aus einem Spaltentitel.
pub fn slugify(title: &str) -> String {
    title
        .trim()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '_')
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("_")
        .to_uppercase()
}

/// Generiert einen URL-freundlichen Slug aus einem Projekttitel.
/// "Mein tolles Projekt!" → "mein-tolles-projekt"
pub fn project_slugify(title: &str) -> String {
    let s: String = title
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'ä' => "ae".to_string(),
            'ö' => "oe".to_string(),
            'ü' => "ue".to_string(),
            'ß' => "ss".to_string(),
            c if c.is_ascii_alphanumeric() => c.to_string(),
            ' ' | '_' | '-' => "-".to_string(),
            _ => String::new(),
        })
        .collect();
    // Collapse multiple hyphens and trim them
    let mut result = String::new();
    let mut prev_hyphen = true; // start true to trim leading hyphens
    for c in s.chars() {
        if c == '-' {
            if !prev_hyphen { result.push('-'); }
            prev_hyphen = true;
        } else {
            result.push(c);
            prev_hyphen = false;
        }
    }
    // Trim trailing hyphen
    if result.ends_with('-') { result.pop(); }
    // Truncate to 60 chars on word boundary
    if result.len() > 60 {
        if let Some(pos) = result[..60].rfind('-') {
            result.truncate(pos);
        } else {
            result.truncate(60);
        }
    }
    result
}

/// Generiert einen eindeutigen Task-Slug innerhalb einer Task-Liste.
pub fn unique_task_slug(title: &str, existing_tasks: &[Task], exclude_id: &str) -> String {
    let base = project_slugify(title);
    let existing: Vec<&str> = existing_tasks.iter()
        .filter(|t| t.id != exclude_id)
        .map(|t| t.slug.as_str())
        .collect();
    if !existing.contains(&base.as_str()) {
        return base;
    }
    for i in 2.. {
        let candidate = format!("{base}-{i}");
        if !existing.contains(&candidate.as_str()) {
            return candidate;
        }
    }
    unreachable!()
}

/// Ein Teammitglied, das Aufgaben zugewiesen bekommen kann.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub id: String,
    pub name: String,
    /// URL oder Initialen-Kürzel für den Avatar.
    pub avatar: String,
    pub role: String,
}

/// Eine einzelne Aufgabe (Karte) im Board.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Task {
    pub id: String,
    /// URL-freundlicher Slug (auto-generiert aus Titel).
    #[serde(default)]
    pub slug: String,
    pub title: String,
    pub description: String,
    /// ID der Spalte, in der sich die Aufgabe befindet.
    pub column_id: String,
    /// Optionaler Slug der Spalte – wird beim Import in column_id aufgelöst.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub column_slug: String,
    /// ID der vorherigen Spalte (für Undo / Audit).
    pub previous_row: String,
    pub assignee_ids: Vec<String>,
    pub labels: Vec<String>,
    /// Reihenfolge innerhalb der Spalte.
    pub order: i32,
    /// Story Points (0-100).
    pub points: i32,
    /// Zugewiesener Bearbeiter.
    pub worker: String,
    /// Erstellt von.
    pub creator: String,
    /// Audit-Log: String (legacy) oder Objekt `{"ts","user","msg"}`.
    pub logs: Vec<serde_json::Value>,
    /// Kommentare: z.B. "Frank: Bitte Prio erhöhen".
    pub comments: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    /// Task-Typ: "task" (Standard), "epic" oder "job".
    pub task_type: String,
    /// IDs der Tasks, die dieser Task blockiert.
    pub blocks: Vec<String>,
    /// IDs der Tasks, die diesen Task blockieren.
    pub blocked_by: Vec<String>,
    /// Parent-Task-ID (für Subtasks eines Epics).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub parent_id: String,
    /// IDs der Subtasks (denormalisiert für schnellen Zugriff auf Epics).
    pub subtask_ids: Vec<String>,
}

fn default_task_type() -> String { "task".to_string() }

/// Erzeugt einen strukturierten Log-Eintrag.
pub fn log_entry(user: &str, msg: &str) -> serde_json::Value {
    serde_json::json!({
        "ts": chrono::Local::now().format("%m-%d %H:%M").to_string(),
        "user": user,
        "msg": msg,
    })
}

impl Default for Task {
    fn default() -> Self {
        Self {
            id: String::new(),
            slug: String::new(),
            title: String::new(),
            description: String::new(),
            column_id: String::new(),
            column_slug: String::new(),
            previous_row: String::new(),
            assignee_ids: vec![],
            labels: vec![],
            order: 0,
            points: 0,
            worker: String::new(),
            creator: String::new(),
            logs: vec![],
            comments: vec![],
            created_at: String::new(),
            updated_at: String::new(),
            task_type: "task".to_string(),
            blocks: vec![],
            blocked_by: vec![],
            parent_id: String::new(),
            subtask_ids: vec![],
        }
    }
}
