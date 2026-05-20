//! Integrationstests für den Velocity-Endpoint `GET /api/projects/:id/stats/velocity`.
//!
//! Feature: `compute_velocity`-Funktion + HTTP-Handler
//! - Gibt pro Woche `{week_start, points_done, tasks_done}` zurück.
//! - Berechnung basierend auf `updated_at` von Tasks in der Done-Spalte.
//! - Default-Range: letzte 8 Wochen.
//! - Tasks ohne Points zählen mit 0.

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use crate::controllers::project_controller::compute_velocity;
    use crate::models::*;
    use crate::services::project_service::default_project;

    // -----------------------------------------------------------------------
    // Test 1: Leeres Projekt → 8 Wochen mit 0 Points
    // -----------------------------------------------------------------------

    /// Ein Projekt ohne Tasks muss 8 Einträge mit 0 zurückgeben (Default-Range).
    #[test]
    fn test_velocity_empty_project_returns_8_weeks() {
        let project = default_project("VelocityEmpty".into());
        let done_col_id = project
            .columns
            .iter()
            .find(|c| c.title == "Done")
            .map(|c| c.id.clone())
            .expect("default project must have Done column");

        let entries = compute_velocity(&project, &done_col_id, 8);

        assert_eq!(entries.len(), 8, "Muss 8 Wochen-Einträge liefern");
        for e in &entries {
            assert_eq!(e.points_done, 0, "Leeres Projekt: points_done muss 0 sein");
            assert_eq!(e.tasks_done, 0, "Leeres Projekt: tasks_done muss 0 sein");
            assert!(!e.week_start.is_empty(), "week_start darf nicht leer sein");
        }
    }

    // -----------------------------------------------------------------------
    // Test 2: Task in Done dieser Woche → Points korrekt summiert
    // -----------------------------------------------------------------------

    /// Ein Task mit 5 Points in Done dieser Woche muss in der letzten Woche erscheinen.
    #[test]
    fn test_velocity_task_done_this_week_counted() {
        let mut project = default_project("VelocityThisWeek".into());
        let done_col_id = project
            .columns
            .iter()
            .find(|c| c.title == "Done")
            .map(|c| c.id.clone())
            .expect("must have Done column");

        // Task mit updated_at = heute in Done-Spalte
        let now = Utc::now().to_rfc3339();
        let task = Task {
            id: "task-done-1".to_string(),
            title: "Done Task".to_string(),
            column_id: done_col_id.clone(),
            points: 5,
            updated_at: now,
            ..Default::default()
        };
        project.tasks.push(task);

        let entries = compute_velocity(&project, &done_col_id, 8);

        assert_eq!(entries.len(), 8);
        // Die letzte (neueste) Woche muss 5 Points haben
        let last = entries.last().expect("must have entries");
        assert_eq!(last.points_done, 5, "Letzte Woche muss 5 Points haben");
        assert_eq!(last.tasks_done, 1, "Letzte Woche muss 1 Task haben");
    }

    // -----------------------------------------------------------------------
    // Test 3: Task ohne Points zählt mit 0
    // -----------------------------------------------------------------------

    /// Tasks ohne Points (points=0) werden mitgezählt (tasks_done), aber points_done bleibt 0.
    #[test]
    fn test_velocity_task_without_points_counted_as_zero() {
        let mut project = default_project("VelocityZeroPoints".into());
        let done_col_id = project
            .columns
            .iter()
            .find(|c| c.title == "Done")
            .map(|c| c.id.clone())
            .expect("must have Done column");

        let now = Utc::now().to_rfc3339();
        let task = Task {
            id: "task-nopoints-1".to_string(),
            title: "No Points Task".to_string(),
            column_id: done_col_id.clone(),
            points: 0,
            updated_at: now,
            ..Default::default()
        };
        project.tasks.push(task);

        let entries = compute_velocity(&project, &done_col_id, 8);

        let last = entries.last().expect("must have entries");
        assert_eq!(last.points_done, 0, "points_done muss 0 sein");
        assert_eq!(
            last.tasks_done, 1,
            "tasks_done muss 1 sein (Task zählt trotzdem)"
        );
    }

    // -----------------------------------------------------------------------
    // Test 4: Tasks in unterschiedlichen Wochen korrekt zugeordnet
    // -----------------------------------------------------------------------

    /// Zwei Tasks in unterschiedlichen Wochen landen in den richtigen Buckets.
    #[test]
    fn test_velocity_multiple_weeks_bucketed_correctly() {
        let mut project = default_project("VelocityMultiWeek".into());
        let done_col_id = project
            .columns
            .iter()
            .find(|c| c.title == "Done")
            .map(|c| c.id.clone())
            .expect("must have Done column");

        let now = Utc::now();
        // Task 1: diese Woche, 3 Points
        let task1 = Task {
            id: "task-week0-1".to_string(),
            title: "Week 0 Task".to_string(),
            column_id: done_col_id.clone(),
            points: 3,
            updated_at: now.to_rfc3339(),
            ..Default::default()
        };
        // Task 2: vor 2 Wochen, 7 Points
        let two_weeks_ago = (now - Duration::weeks(2)).to_rfc3339();
        let task2 = Task {
            id: "task-week2-1".to_string(),
            title: "Week 2 Task".to_string(),
            column_id: done_col_id.clone(),
            points: 7,
            updated_at: two_weeks_ago,
            ..Default::default()
        };
        project.tasks.push(task1);
        project.tasks.push(task2);

        let entries = compute_velocity(&project, &done_col_id, 8);

        assert_eq!(entries.len(), 8);

        // Letzte Woche (neueste): 3 Points
        let last = entries.last().unwrap();
        assert_eq!(last.points_done, 3, "Neueste Woche muss 3 Points haben");

        // Vor 2 Wochen: 7 Points (Index von hinten: entries[5])
        let week2 = &entries[entries.len() - 3];
        assert_eq!(week2.points_done, 7, "Woche -2 muss 7 Points haben");
    }

    // -----------------------------------------------------------------------
    // Test 5: N=4 → genau 4 Einträge
    // -----------------------------------------------------------------------

    /// Mit weeks=4 werden genau 4 Einträge zurückgegeben.
    #[test]
    fn test_velocity_custom_weeks_count() {
        let project = default_project("VelocityN4".into());
        let done_col_id = project
            .columns
            .iter()
            .find(|c| c.title == "Done")
            .map(|c| c.id.clone())
            .expect("must have Done column");

        let entries = compute_velocity(&project, &done_col_id, 4);

        assert_eq!(entries.len(), 4, "Muss genau 4 Einträge liefern");
    }

    // -----------------------------------------------------------------------
    // Test 6: week_start ist ISO-Datum im Format YYYY-MM-DD
    // -----------------------------------------------------------------------

    /// week_start muss ein valides ISO-Datum (YYYY-MM-DD) sein.
    #[test]
    fn test_velocity_week_start_format() {
        let project = default_project("VelocityDateFormat".into());
        let done_col_id = project
            .columns
            .iter()
            .find(|c| c.title == "Done")
            .map(|c| c.id.clone())
            .expect("must have Done column");

        let entries = compute_velocity(&project, &done_col_id, 8);

        for e in &entries {
            // Format: YYYY-MM-DD
            assert_eq!(
                e.week_start.len(),
                10,
                "week_start '{}' muss 10 Zeichen lang sein (YYYY-MM-DD)",
                e.week_start
            );
            assert!(
                e.week_start.chars().nth(4) == Some('-')
                    && e.week_start.chars().nth(7) == Some('-'),
                "week_start '{}' muss Format YYYY-MM-DD haben",
                e.week_start
            );
        }
    }

    // -----------------------------------------------------------------------
    // Test 7: Tasks außerhalb des Zeitfensters werden nicht gezählt
    // -----------------------------------------------------------------------

    /// Ein Task von vor 10 Wochen wird bei weeks=8 nicht mitgezählt.
    #[test]
    fn test_velocity_old_tasks_excluded() {
        let mut project = default_project("VelocityOldTask".into());
        let done_col_id = project
            .columns
            .iter()
            .find(|c| c.title == "Done")
            .map(|c| c.id.clone())
            .expect("must have Done column");

        let ten_weeks_ago = (Utc::now() - Duration::weeks(10)).to_rfc3339();
        let task = Task {
            id: "task-old-1".to_string(),
            title: "Old Task".to_string(),
            column_id: done_col_id.clone(),
            points: 99,
            updated_at: ten_weeks_ago,
            ..Default::default()
        };
        project.tasks.push(task);

        let entries = compute_velocity(&project, &done_col_id, 8);

        let total_points: i32 = entries.iter().map(|e| e.points_done).sum();
        assert_eq!(
            total_points, 0,
            "Task von vor 10 Wochen darf nicht gezählt werden"
        );
    }

    // -----------------------------------------------------------------------
    // Test 8: Tasks in anderen Spalten werden nicht gezählt
    // -----------------------------------------------------------------------

    /// Tasks die NICHT in Done sind, werden ignoriert.
    #[test]
    fn test_velocity_non_done_tasks_excluded() {
        let mut project = default_project("VelocityNonDone".into());
        let done_col_id = project
            .columns
            .iter()
            .find(|c| c.title == "Done")
            .map(|c| c.id.clone())
            .expect("must have Done column");

        let todo_col_id = project
            .columns
            .iter()
            .find(|c| c.title != "Done" && !c.hidden)
            .map(|c| c.id.clone())
            .expect("must have non-Done column");

        let now = Utc::now().to_rfc3339();
        let task = Task {
            id: "task-todo-1".to_string(),
            title: "Todo Task".to_string(),
            column_id: todo_col_id,
            points: 10,
            updated_at: now,
            ..Default::default()
        };
        project.tasks.push(task);

        let entries = compute_velocity(&project, &done_col_id, 8);

        let total_points: i32 = entries.iter().map(|e| e.points_done).sum();
        assert_eq!(
            total_points, 0,
            "Nicht-Done-Tasks dürfen nicht gezählt werden"
        );
    }
}
