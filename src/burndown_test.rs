//! Tests für den Burndown-Chart-Endpoint `GET /api/projects/:id/stats/burndown`.
//!
//! Feature: `compute_burndown`-Funktion
//! - Liefert Zeitreihe `[{date, remaining_tasks, remaining_points, ideal_tasks, ideal_points}]`
//! - Eine Zeile pro Tag von `from` bis `to` (inkl.)
//! - Basis: Tasks die in der Done-Spalte sind, zählen als „erledigt" ab dem Tag,
//!   an dem `updated_at` in den Zeitraum fällt.
//! - Ideal-Linie: linearer Abbau von Anfang bis Ende.
//! - Edge Case: leerer Datumsbereich → leere Zeitreihe.
//! - Performance: 90-Tage-Range mit 100 Tasks < 200 ms.

#[cfg(test)]
mod tests {
    use chrono::{Duration, NaiveDate, Utc};

    use crate::controllers::project_controller::compute_burndown;
    use crate::models::*;
    use crate::services::project_service::default_project;

    // -----------------------------------------------------------------------
    // Hilfsfunktion: Erstellt einen Task in Done-Spalte mit gesetztem updated_at
    // -----------------------------------------------------------------------
    fn make_done_task(id: &str, done_col_id: &str, updated_at: &str, points: i32) -> Task {
        Task {
            id: id.to_string(),
            title: format!("Task {id}"),
            column_id: done_col_id.to_string(),
            points,
            updated_at: updated_at.to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            ..Default::default()
        }
    }

    fn make_todo_task(id: &str, todo_col_id: &str, points: i32) -> Task {
        Task {
            id: id.to_string(),
            title: format!("Task {id}"),
            column_id: todo_col_id.to_string(),
            points,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            ..Default::default()
        }
    }

    // -----------------------------------------------------------------------
    // Test 1: Leere Range → leere Zeitreihe
    // -----------------------------------------------------------------------

    /// Eine Range wo `from` > `to` ergibt eine leere Zeitreihe.
    #[test]
    fn test_burndown_empty_range_returns_empty() {
        let project = default_project("BurndownEmpty".into());
        let done_col_id = project
            .columns
            .iter()
            .find(|c| c.title == "Done")
            .map(|c| c.id.clone())
            .expect("must have Done column");

        let from = NaiveDate::from_ymd_opt(2026, 5, 10).unwrap();
        let to = NaiveDate::from_ymd_opt(2026, 5, 5).unwrap(); // before from

        let result = compute_burndown(&project, &done_col_id, from, to);
        assert!(
            result.is_empty(),
            "Umgekehrte Range muss leere Zeitreihe liefern"
        );
    }

    // -----------------------------------------------------------------------
    // Test 2: Single-Day-Range → genau 1 Eintrag
    // -----------------------------------------------------------------------

    /// Eine Range von einem Tag liefert genau 1 Eintrag.
    #[test]
    fn test_burndown_single_day_range() {
        let project = default_project("BurndownSingleDay".into());
        let done_col_id = project
            .columns
            .iter()
            .find(|c| c.title == "Done")
            .map(|c| c.id.clone())
            .expect("must have Done column");

        let from = NaiveDate::from_ymd_opt(2026, 5, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2026, 5, 1).unwrap();

        let result = compute_burndown(&project, &done_col_id, from, to);
        assert_eq!(
            result.len(),
            1,
            "Single-Day-Range muss genau 1 Eintrag liefern"
        );
        assert_eq!(result[0].date, "2026-05-01");
    }

    // -----------------------------------------------------------------------
    // Test 3: Leeres Projekt → alle remaining = Gesamtzahl (0 Tasks = 0)
    // -----------------------------------------------------------------------

    /// Leeres Projekt (keine Tasks): remaining_tasks und remaining_points sind immer 0.
    #[test]
    fn test_burndown_empty_project_all_zeros() {
        let project = default_project("BurndownZero".into());
        let done_col_id = project
            .columns
            .iter()
            .find(|c| c.title == "Done")
            .map(|c| c.id.clone())
            .expect("must have Done column");

        let from = NaiveDate::from_ymd_opt(2026, 5, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2026, 5, 7).unwrap();

        let result = compute_burndown(&project, &done_col_id, from, to);
        assert_eq!(result.len(), 7, "7-Tage-Range muss 7 Einträge liefern");
        for entry in &result {
            assert_eq!(entry.remaining_tasks, 0, "Kein Task → remaining_tasks = 0");
            assert_eq!(
                entry.remaining_points, 0,
                "Kein Task → remaining_points = 0"
            );
        }
    }

    // -----------------------------------------------------------------------
    // Test 4: Task abgeschlossen innerhalb der Range → ab diesem Tag weniger remaining
    // -----------------------------------------------------------------------

    /// 1 Task mit 3 Points in Todo + 1 Done-Task (abgeschlossen Tag 3 der Range):
    /// - Tage 1-2: remaining_tasks = 2, remaining_points = 8
    /// - Ab Tag 3: remaining_tasks = 1, remaining_points = 5
    #[test]
    fn test_burndown_task_done_reduces_remaining() {
        let mut project = default_project("BurndownDoneReduces".into());
        let done_col_id = project
            .columns
            .iter()
            .find(|c| c.title == "Done")
            .map(|c| c.id.clone())
            .expect("must have Done column");
        let todo_col_id = project
            .columns
            .iter()
            .find(|c| c.title == "Todo")
            .map(|c| c.id.clone())
            .expect("must have Todo column");

        // 2 offene Tasks
        project
            .tasks
            .push(make_todo_task("todo-1", &todo_col_id, 5));
        project
            .tasks
            .push(make_todo_task("todo-2", &todo_col_id, 3));
        // 1 Task am 3. Mai erledigt
        project.tasks.push(make_done_task(
            "done-1",
            &done_col_id,
            "2026-05-03T12:00:00Z",
            3,
        ));

        let from = NaiveDate::from_ymd_opt(2026, 5, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2026, 5, 5).unwrap();

        let result = compute_burndown(&project, &done_col_id, from, to);
        assert_eq!(result.len(), 5);

        // Tag 1 (2026-05-01): noch nichts erledigt → 3 Tasks remaining (2 todo + 1 done der noch nicht "abgehakt" ist)
        // Genauer: remaining = alle Tasks - Tasks die bis zu diesem Datum done wurden
        // Am 1. Mai: done-1 noch nicht fertig → remaining = 3 Tasks, 11 Points
        assert_eq!(result[0].remaining_tasks, 3, "Tag 1: 3 Tasks remaining");
        assert_eq!(result[0].remaining_points, 11, "Tag 1: 11 Points remaining");

        // Tag 3 (2026-05-03): done-1 fertig am 3. Mai → remaining = 2 Tasks, 8 Points
        assert_eq!(result[2].remaining_tasks, 2, "Tag 3: 2 Tasks remaining");
        assert_eq!(result[2].remaining_points, 8, "Tag 3: 8 Points remaining");

        // Tag 5: immer noch 2 Tasks remaining
        assert_eq!(result[4].remaining_tasks, 2, "Tag 5: 2 Tasks remaining");
    }

    // -----------------------------------------------------------------------
    // Test 5: Ideal-Linie
    // -----------------------------------------------------------------------

    /// Bei einem Projekt mit 4 Tasks und 4-Tage-Range:
    /// Ideal startet bei total_tasks/total_points und fällt linear auf 0.
    #[test]
    fn test_burndown_ideal_line_linear() {
        let mut project = default_project("BurndownIdeal".into());
        let done_col_id = project
            .columns
            .iter()
            .find(|c| c.title == "Done")
            .map(|c| c.id.clone())
            .expect("must have Done column");
        let todo_col_id = project
            .columns
            .iter()
            .find(|c| c.title == "Todo")
            .map(|c| c.id.clone())
            .expect("must have Todo column");

        // 4 Tasks mit je 2 Points
        for i in 1..=4 {
            project
                .tasks
                .push(make_todo_task(&format!("todo-{i}"), &todo_col_id, 2));
        }

        let from = NaiveDate::from_ymd_opt(2026, 5, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2026, 5, 4).unwrap(); // 4 Tage

        let result = compute_burndown(&project, &done_col_id, from, to);
        assert_eq!(result.len(), 4);

        // Ideal: Tag 0 = 4 Tasks / 8 Points, Tag 3 = 0 Tasks / 0 Points
        // Intervall = (total) / (days - 1)
        // Tag 0 (i=0): ideal = 4.0 tasks, 8.0 points
        // Tag 1 (i=1): ideal = ~2.67 tasks → 2.67, etc.
        assert_eq!(
            result[0].ideal_tasks, 4.0,
            "Ideal Startpunkt muss total_tasks sein"
        );
        assert_eq!(
            result[0].ideal_points, 8.0,
            "Ideal Startpunkt muss total_points sein"
        );
        assert_eq!(result[3].ideal_tasks, 0.0, "Ideal Endpunkt muss 0 sein");
        assert_eq!(result[3].ideal_points, 0.0, "Ideal Endpunkt muss 0 sein");
    }

    // -----------------------------------------------------------------------
    // Test 6: Tasks außerhalb der Range werden korrekt behandelt
    // -----------------------------------------------------------------------

    /// Task der VOR der Range erledigt wurde zählt ab Tag 1 als bereits done.
    #[test]
    fn test_burndown_task_done_before_range_already_excluded() {
        let mut project = default_project("BurndownBeforeRange".into());
        let done_col_id = project
            .columns
            .iter()
            .find(|c| c.title == "Done")
            .map(|c| c.id.clone())
            .expect("must have Done column");

        // Task vor der Range erledigt
        project.tasks.push(make_done_task(
            "done-before",
            &done_col_id,
            "2026-04-01T12:00:00Z",
            10,
        ));

        let from = NaiveDate::from_ymd_opt(2026, 5, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2026, 5, 3).unwrap();

        let result = compute_burndown(&project, &done_col_id, from, to);
        assert_eq!(result.len(), 3);

        // Vor der Range erledigt → ab Tag 1 schon 0 remaining
        assert_eq!(
            result[0].remaining_tasks, 0,
            "Vor Range erledigter Task: remaining_tasks Tag 1 = 0"
        );
        assert_eq!(
            result[0].remaining_points, 0,
            "Vor Range erledigter Task: remaining_points Tag 1 = 0"
        );
    }

    // -----------------------------------------------------------------------
    // Test 7: date-Format YYYY-MM-DD
    // -----------------------------------------------------------------------

    /// Alle date-Felder im Ergebnis müssen Format YYYY-MM-DD haben.
    #[test]
    fn test_burndown_date_format() {
        let project = default_project("BurndownDateFmt".into());
        let done_col_id = project
            .columns
            .iter()
            .find(|c| c.title == "Done")
            .map(|c| c.id.clone())
            .expect("must have Done column");

        let from = NaiveDate::from_ymd_opt(2026, 5, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2026, 5, 7).unwrap();

        let result = compute_burndown(&project, &done_col_id, from, to);
        for entry in &result {
            assert_eq!(
                entry.date.len(),
                10,
                "date '{}' muss 10 Zeichen lang sein",
                entry.date
            );
            assert!(
                entry.date.chars().nth(4) == Some('-') && entry.date.chars().nth(7) == Some('-'),
                "date '{}' muss Format YYYY-MM-DD haben",
                entry.date
            );
        }
    }

    // -----------------------------------------------------------------------
    // Test 8: Performance – 90 Tage, 100 Tasks < 200 ms
    // -----------------------------------------------------------------------

    /// Berechnung für 90-Tage-Range mit 100 Tasks muss unter 200 ms dauern.
    #[test]
    fn test_burndown_performance_90days_100tasks() {
        let mut project = default_project("BurndownPerf".into());
        let done_col_id = project
            .columns
            .iter()
            .find(|c| c.title == "Done")
            .map(|c| c.id.clone())
            .expect("must have Done column");

        let now = Utc::now();
        // 100 Tasks gleichmäßig über die letzten 90 Tage verteilt
        for i in 0..100_u32 {
            let days_ago = (i % 90) as i64;
            let ts = (now - Duration::days(days_ago)).to_rfc3339();
            project.tasks.push(make_done_task(
                &format!("perf-{i}"),
                &done_col_id,
                &ts,
                (i % 10 + 1) as i32,
            ));
        }

        let to = now.date_naive();
        let from = to - Duration::days(89);

        let start = std::time::Instant::now();
        let result = compute_burndown(&project, &done_col_id, from, to);
        let elapsed = start.elapsed();

        assert_eq!(result.len(), 90, "90-Tage-Range muss 90 Einträge liefern");
        assert!(
            elapsed.as_millis() < 200,
            "compute_burndown darf max. 200ms dauern, war: {}ms",
            elapsed.as_millis()
        );
    }
}
