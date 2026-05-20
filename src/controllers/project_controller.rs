// Handler für Projekt-CRUD.

use std::cmp::Reverse;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ApiError;
use crate::models::*;
use crate::services::publish_update;
use crate::state::AppState;

/// Ein Eintrag im Stats-Columns-Response: Spalten-Metadaten + Task-Count.
#[derive(Debug, Serialize, Clone)]
pub struct ColumnStat {
    pub column_id: String,
    pub title: String,
    pub task_count: usize,
}

/// Berechnet für jede sichtbare Spalte die Anzahl Tasks.
/// Archivierte (hidden) Spalten werden übersprungen.
pub fn compute_column_stats(project: &ProjectDoc) -> Vec<ColumnStat> {
    project
        .columns
        .iter()
        .filter(|c| !c.hidden)
        .map(|col| {
            let task_count = project
                .tasks
                .iter()
                .filter(|t| t.column_id == col.id)
                .count();
            ColumnStat {
                column_id: col.id.clone(),
                title: col.title.clone(),
                task_count,
            }
        })
        .collect()
}

/// GET /api/projects/:id/stats/columns – Anzahl Tasks pro sichtbarer Spalte.
pub async fn project_stats_columns(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<ColumnStat>>, ApiError> {
    let project = state.store.resolve_project(&id).await?;
    let stats = compute_column_stats(&project);
    Ok(Json(stats))
}

/// GET /api/projects – Alle Projekte auflisten (sortiert nach `order`).
pub async fn list_projects(
    State(state): State<AppState>,
) -> Result<Json<Vec<ProjectDoc>>, ApiError> {
    let mut projects = state.store.list_projects().await?;
    // Auto-migrate: Slugs generieren für Projekte ohne Slug.
    let mut existing_slugs: Vec<String> = projects
        .iter()
        .filter(|p| !p.slug.is_empty())
        .map(|p| p.slug.clone())
        .collect();
    for project in &mut projects {
        if project.slug.is_empty() {
            let base_slug = project_slugify(&project.title);
            let slug_refs: Vec<&str> = existing_slugs.iter().map(|s| s.as_str()).collect();
            project.slug = unique_slug(&base_slug, &slug_refs);
            existing_slugs.push(project.slug.clone());
            let _ = state.store.put_project(project.clone()).await;
        }
    }
    // Stabile Sortierung nach `order`-Feld (aufsteigend).
    projects.sort_by_key(|p| p.order);
    Ok(Json(projects))
}

/// Request-Body für POST /api/projects/reorder.
#[derive(Debug, Deserialize)]
pub struct ReorderProjectsBody {
    pub ids: Vec<String>,
}

/// POST /api/projects/reorder – Projekt-Reihenfolge in der Sidebar persistieren.
///
/// Erwartet `{"ids": ["uuid1", "uuid2", ...]}` in der gewünschten Reihenfolge.
/// Setzt für jedes bekannte Projekt `order = index` und speichert es.
/// Unbekannte IDs werden übersprungen.
pub async fn reorder_projects(
    State(state): State<AppState>,
    Json(payload): Json<ReorderProjectsBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    for (index, id) in payload.ids.iter().enumerate() {
        match state.store.get_project(id).await {
            Ok(mut project) => {
                project.order = index as i32;
                state.store.put_project(project).await?;
            }
            // Unbekannte ID überspringen
            Err(ApiError::NotFound(_)) => {}
            Err(e) => return Err(e),
        }
    }
    Ok(Json(serde_json::json!({"ok": true})))
}

/// POST /api/projects – Neues Projekt anlegen.
pub async fn create_project(
    State(state): State<AppState>,
    Json(mut payload): Json<ProjectDoc>,
) -> Result<Json<ProjectDoc>, ApiError> {
    if payload.id.is_empty() {
        payload.id = Uuid::new_v4().to_string();
    }
    payload.rev = None;
    // Auto-generate project slug from title
    if payload.slug.is_empty() {
        let base_slug = project_slugify(&payload.title);
        let existing = state.store.list_projects().await?;
        let existing_slugs: Vec<&str> = existing.iter().map(|p| p.slug.as_str()).collect();
        payload.slug = unique_slug(&base_slug, &existing_slugs);
    }
    // Ensure all columns have slugs
    for col in &mut payload.columns {
        if col.slug.is_empty() {
            col.slug = slugify(&col.title);
        }
    }
    let created = state.store.create_project(payload).await?;
    publish_update(&state, &created.id).await;
    Ok(Json(created))
}

/// Erzeugt einen eindeutigen Slug durch Anhängen eines Zählers.
fn unique_slug(base: &str, existing: &[&str]) -> String {
    if !existing.contains(&base) {
        return base.to_string();
    }
    for i in 2.. {
        let candidate = format!("{base}-{i}");
        if !existing.contains(&candidate.as_str()) {
            return candidate;
        }
    }
    unreachable!()
}

/// GET /api/projects/:id – Ein Projekt abrufen (akzeptiert UUID oder Slug).
pub async fn get_project(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<GetProjectQuery>,
) -> Result<Json<ProjectDoc>, ApiError> {
    let mut project = state.store.resolve_project(&id).await?;
    // Auto-migrate: Slug generieren für Projekte ohne Slug.
    if project.slug.is_empty() {
        let base_slug = project_slugify(&project.title);
        let existing = state.store.list_projects().await?;
        let existing_slugs: Vec<&str> = existing
            .iter()
            .filter(|p| p.id != project.id)
            .map(|p| p.slug.as_str())
            .collect();
        project.slug = unique_slug(&base_slug, &existing_slugs);
        let _ = state.store.put_project(project.clone()).await;
    }
    // Auto-migrate: Task-Slugs generieren für Tasks ohne Slug.
    let needs_task_slugs = project.tasks.iter().any(|t| t.slug.is_empty());
    if needs_task_slugs {
        let mut existing_slugs: Vec<String> = Vec::new();
        for task in &mut project.tasks {
            if task.slug.is_empty() {
                let base = project_slugify(&task.title);
                let refs: Vec<&str> = existing_slugs.iter().map(|s| s.as_str()).collect();
                task.slug = unique_slug(&base, &refs);
            }
            existing_slugs.push(task.slug.clone());
        }
        let _ = state.store.put_project(project.clone()).await;
    }
    if !query.include_archived {
        let hidden_col_ids: Vec<String> = project
            .columns
            .iter()
            .filter(|c| c.hidden)
            .map(|c| c.id.clone())
            .collect();
        project
            .tasks
            .retain(|t| !hidden_col_ids.contains(&t.column_id));
        project.columns.retain(|c| !c.hidden);
    }
    // Sort tasks based on query parameter (default: order).
    match query.sort.as_deref() {
        Some("title") => project.tasks.sort_by_key(|t| t.title.to_lowercase()),
        Some("created") => project.tasks.sort_by_key(|t| t.created_at.clone()),
        Some("updated") => project
            .tasks
            .sort_by(|a, b| b.updated_at.cmp(&a.updated_at)),
        Some("points") => project.tasks.sort_by_key(|t| Reverse(t.points)),
        _ => project.tasks.sort_by_key(|t| t.order),
    }
    // Group epics with their subtasks: epic first, then its children in order.
    if query.group_epics {
        let mut grouped: Vec<Task> = Vec::with_capacity(project.tasks.len());
        let mut placed: std::collections::HashSet<String> = std::collections::HashSet::new();
        // Collect subtask IDs for quick lookup.
        let subtask_parent: std::collections::HashMap<String, String> = project
            .tasks
            .iter()
            .filter(|t| !t.parent_id.is_empty())
            .map(|t| (t.id.clone(), t.parent_id.clone()))
            .collect();
        for task in &project.tasks {
            if placed.contains(&task.id) {
                continue;
            }
            // Skip subtasks here; they'll be placed after their parent.
            if subtask_parent.contains_key(&task.id) {
                continue;
            }
            grouped.push(task.clone());
            placed.insert(task.id.clone());
            // If this is an epic, insert its subtasks right after.
            if !task.subtask_ids.is_empty() {
                // Maintain order among subtasks.
                let mut subs: Vec<&Task> = project
                    .tasks
                    .iter()
                    .filter(|t| t.parent_id == task.id && !placed.contains(&t.id))
                    .collect();
                subs.sort_by_key(|t| t.order);
                for sub in subs {
                    grouped.push(sub.clone());
                    placed.insert(sub.id.clone());
                }
            }
        }
        // Append any remaining tasks (orphaned subtasks whose parent wasn't found).
        for task in &project.tasks {
            if !placed.contains(&task.id) {
                grouped.push(task.clone());
            }
        }
        project.tasks = grouped;
    }
    Ok(Json(project))
}

/// PUT /api/projects/:id – Vollständiges Projekt ersetzen (akzeptiert UUID oder Slug).
pub async fn update_project(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(mut payload): Json<ProjectDoc>,
) -> Result<Json<ProjectDoc>, ApiError> {
    let real_id = state.store.resolve_project_id(&id).await?;
    payload.id = real_id.clone();
    let current = state.store.get_project(&real_id).await?;
    payload.rev = current.rev;
    // Re-generate slug if title changed
    if payload.slug.is_empty() || current.title != payload.title {
        let base_slug = project_slugify(&payload.title);
        let existing = state.store.list_projects().await?;
        let existing_slugs: Vec<&str> = existing
            .iter()
            .filter(|p| p.id != real_id)
            .map(|p| p.slug.as_str())
            .collect();
        payload.slug = unique_slug(&base_slug, &existing_slugs);
    }
    let updated = state.store.put_project(payload).await?;
    publish_update(&state, &real_id).await;
    Ok(Json(updated))
}

/// DELETE /api/projects/:id?rev=<rev> – Projekt löschen (akzeptiert UUID oder Slug).
pub async fn delete_project(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<DeleteQuery>,
) -> Result<StatusCode, ApiError> {
    let real_id = state.store.resolve_project_id(&id).await?;
    state.store.delete_project(&real_id, &query.rev).await?;
    publish_update(&state, &real_id).await;
    Ok(StatusCode::NO_CONTENT)
}

/// Ein Eintrag in der Burndown-Chart-Zeitreihe.
#[derive(Debug, Serialize, Clone)]
pub struct BurndownEntry {
    /// Datum im Format YYYY-MM-DD.
    pub date: String,
    /// Anzahl noch nicht erledigter Tasks.
    pub remaining_tasks: i32,
    /// Summe der Story Points noch nicht erledigter Tasks.
    pub remaining_points: i32,
    /// Idealer Verlauf (Tasks).
    pub ideal_tasks: f64,
    /// Idealer Verlauf (Points).
    pub ideal_points: f64,
}

/// Berechnet eine Burndown-Chart-Zeitreihe für ein Projekt.
///
/// - Eine Zeile pro Tag von `from` bis `to` (inkl.).
/// - `remaining_tasks`/`remaining_points`: alle Tasks minus jene die bis zu diesem Tag
///   in die Done-Spalte (`done_col_id`) verschoben wurden (`updated_at ≤ tag`).
/// - Ideal-Linie: linearer Abbau von Gesamt-Tasks/Points (Tag 0) auf 0 (letzter Tag).
/// - Leere Range (`from > to`) → leere Zeitreihe.
pub fn compute_burndown(
    project: &ProjectDoc,
    done_col_id: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Vec<BurndownEntry> {
    if from > to {
        return vec![];
    }

    let total_tasks = project.tasks.len() as i32;
    let total_points: i32 = project.tasks.iter().map(|t| t.points).sum();
    let days = (to - from).num_days() + 1; // inklusive

    // Für jeden Done-Task: Datum an dem er erledigt wurde (updated_at-Date).
    let done_dates: Vec<NaiveDate> = project
        .tasks
        .iter()
        .filter(|t| t.column_id == done_col_id)
        .filter_map(|t| {
            if t.updated_at.is_empty() {
                None
            } else {
                // updated_at ist RFC3339: "2026-05-03T12:00:00Z"
                t.updated_at
                    .parse::<DateTime<Utc>>()
                    .ok()
                    .map(|dt| dt.date_naive())
            }
        })
        .collect();

    let done_points: Vec<(NaiveDate, i32)> = project
        .tasks
        .iter()
        .filter(|t| t.column_id == done_col_id)
        .filter_map(|t| {
            if t.updated_at.is_empty() {
                None
            } else {
                t.updated_at
                    .parse::<DateTime<Utc>>()
                    .ok()
                    .map(|dt| (dt.date_naive(), t.points))
            }
        })
        .collect();

    let mut result = Vec::with_capacity(days as usize);

    for i in 0..days {
        let day = from + Duration::days(i);

        // Erledigte Tasks bis einschließlich diesem Tag
        let done_count = done_dates.iter().filter(|&&d| d <= day).count() as i32;
        let done_pts: i32 = done_points
            .iter()
            .filter(|(d, _)| *d <= day)
            .map(|(_, pts)| pts)
            .sum();

        let remaining_tasks = (total_tasks - done_count).max(0);
        let remaining_points = (total_points - done_pts).max(0);

        // Ideal-Linie: linear von total → 0
        let ideal_tasks = if days <= 1 {
            0.0
        } else {
            total_tasks as f64 * (1.0 - i as f64 / (days - 1) as f64)
        };
        let ideal_points = if days <= 1 {
            0.0
        } else {
            total_points as f64 * (1.0 - i as f64 / (days - 1) as f64)
        };

        result.push(BurndownEntry {
            date: day.format("%Y-%m-%d").to_string(),
            remaining_tasks,
            remaining_points,
            ideal_tasks,
            ideal_points,
        });
    }

    result
}

// ─── Velocity Stats ──────────────────────────────────────────────────────────

/// Ein Velocity-Eintrag: eine Woche mit erledigten Points und Tasks.
#[derive(Debug, Serialize, Clone)]
pub struct VelocityEntry {
    /// ISO-Datum des Wochenanfangs (Montag), Format: YYYY-MM-DD.
    pub week_start: String,
    /// Summe der Story-Points aller in dieser Woche erledigten Tasks.
    pub points_done: i32,
    /// Anzahl der in dieser Woche erledigten Tasks.
    pub tasks_done: i32,
}

/// Query-Parameter für den Velocity-Endpoint.
#[derive(Debug, Deserialize, Default)]
pub struct VelocityQuery {
    /// Anzahl der Wochen. Default: 8.
    pub weeks: Option<u32>,
}

/// Berechnet die wöchentliche Velocity für ein Projekt.
///
/// Iteriert über alle Tasks in der Done-Spalte und ordnet sie anhand von
/// `updated_at` den jeweiligen Kalenderwochen zu.
/// Das Ergebnis ist aufsteigend nach `week_start` sortiert (älteste Woche zuerst).
pub fn compute_velocity(project: &ProjectDoc, done_col_id: &str, weeks: u32) -> Vec<VelocityEntry> {
    let now = Utc::now();
    let weeks = weeks.max(1);

    // Wochenanfang (Montag) für eine gegebene Woche berechnen.
    // ISO-Woche beginnt Montag.
    fn week_monday(date: NaiveDate) -> NaiveDate {
        // Weekday::Mon = 0 days from Monday
        let dow = date.weekday().num_days_from_monday(); // 0=Mo, 6=So
        date - chrono::Duration::days(dow as i64)
    }

    // Wochenanfang dieser Woche (heute).
    let today = now.date_naive();
    let current_week_start = week_monday(today);

    // Bucket-Map: week_start (NaiveDate) → (points_done, tasks_done)
    let mut buckets: std::collections::HashMap<NaiveDate, (i32, i32)> =
        std::collections::HashMap::new();

    // Alle `weeks` Buckets vorinitialisieren (0, 0).
    for w in 0..weeks {
        let monday = current_week_start - Duration::weeks(w as i64);
        buckets.insert(monday, (0, 0));
    }

    // Done-Tasks den Buckets zuordnen.
    let window_start = current_week_start - Duration::weeks((weeks - 1) as i64);
    for task in &project.tasks {
        if task.column_id != done_col_id {
            continue;
        }
        if task.updated_at.is_empty() {
            continue;
        }
        // updated_at parsen (RFC3339).
        let ts = match DateTime::parse_from_rfc3339(&task.updated_at) {
            Ok(dt) => dt.with_timezone(&Utc),
            Err(_) => continue,
        };
        let task_date = ts.date_naive();
        let task_monday = week_monday(task_date);

        // Nur Tasks innerhalb des Zeitfensters berücksichtigen.
        if task_monday < window_start || task_monday > current_week_start {
            continue;
        }

        let entry = buckets.entry(task_monday).or_insert((0, 0));
        entry.0 += task.points;
        entry.1 += 1;
    }

    // Sortiert nach Datum (aufsteigend) → Vec<VelocityEntry>
    let mut result: Vec<(NaiveDate, (i32, i32))> = buckets.into_iter().collect();
    result.sort_by_key(|(date, _)| *date);

    result
        .into_iter()
        .map(|(monday, (points, tasks))| VelocityEntry {
            week_start: monday.format("%Y-%m-%d").to_string(),
            points_done: points,
            tasks_done: tasks,
        })
        .collect()
}

// ─── Burndown Stats ──────────────────────────────────────────────────────────

/// Query-Parameter für den Burndown-Endpoint.
#[derive(Debug, Deserialize, Default)]
pub struct BurndownQuery {
    /// Startdatum (YYYY-MM-DD). Default: 30 Tage vor heute.
    pub from: Option<String>,
    /// Enddatum (YYYY-MM-DD). Default: heute.
    pub to: Option<String>,
}

/// GET /api/projects/:id/stats/burndown?from=YYYY-MM-DD&to=YYYY-MM-DD
///
/// Liefert eine tägliche Burndown-Zeitreihe mit remaining_tasks, remaining_points
/// und linearer Ideal-Linie. Default-Range: letzte 30 Tage.
pub async fn project_stats_burndown(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<BurndownQuery>,
) -> Result<Json<Vec<BurndownEntry>>, ApiError> {
    let project = state.store.resolve_project(&id).await?;

    let today = Utc::now().date_naive();

    let to = query
        .to
        .as_deref()
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .unwrap_or(today);

    let from = query
        .from
        .as_deref()
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .unwrap_or_else(|| to - Duration::days(29));

    // Done-Spalte suchen (Titel "Done", case-insensitive fallback).
    let done_col_id = project
        .columns
        .iter()
        .find(|c| c.title == "Done")
        .or_else(|| {
            project
                .columns
                .iter()
                .find(|c| c.title.to_lowercase() == "done")
        })
        .map(|c| c.id.clone())
        .unwrap_or_default();

    let entries = compute_burndown(&project, &done_col_id, from, to);
    Ok(Json(entries))
}

/// GET /api/projects/:id/stats/velocity?weeks=N – Wöchentliche Velocity.
///
/// Findet die Done-Spalte (Titel = "Done") des Projekts und berechnet
/// für jede der letzten N Wochen (Default: 8) die summierten Story-Points
/// und Task-Anzahl der Tasks, die in dieser Woche zuletzt aktualisiert wurden
/// und sich in der Done-Spalte befinden.
pub async fn project_stats_velocity(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<VelocityQuery>,
) -> Result<Json<Vec<VelocityEntry>>, ApiError> {
    let project = state.store.resolve_project(&id).await?;
    let weeks = query.weeks.unwrap_or(8);

    // Done-Spalte suchen (Titel "Done", case-insensitive fallback).
    let done_col_id = project
        .columns
        .iter()
        .find(|c| c.title == "Done")
        .or_else(|| {
            project
                .columns
                .iter()
                .find(|c| c.title.to_lowercase() == "done")
        })
        .map(|c| c.id.clone())
        .unwrap_or_default();

    let entries = compute_velocity(&project, &done_col_id, weeks);
    Ok(Json(entries))
}
