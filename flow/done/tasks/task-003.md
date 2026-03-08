# Task: Task-Struct erweitern + API-Handler anpassen

**ID:** task-003
**Epic:** epic-002
**Status:** done
**Erstellt:** 2026-03-08
**Assignee:** developer

## Beschreibung
Das Task-Struct um die fehlenden Felder erweitern: `previous_row`, `points`, `worker`, `creator`, `logs`, `comments`. Alle bestehenden API-Handler müssen mit dem erweiterten Modell korrekt arbeiten. Serde-Defaults für Rückwärtskompatibilität mit bestehenden JSON-Dateien.

## Anforderungen
- [ ] Task-Struct enthält: previous_row (String), points (i32), worker (String), creator (String), logs (Vec<String>), comments (Vec<String>)
- [ ] Alle neuen Felder haben Serde-Defaults (`#[serde(default)]`) für Kompatibilität
- [ ] `create_task` Handler setzt `creator` auf "anonymous" wenn leer
- [ ] `update_task` Handler übernimmt alle neuen Felder korrekt
- [ ] `move_task` Handler setzt `previous_row` auf alten `column_id` Wert und schreibt Log-Eintrag
- [ ] MCP `create_task` und `update_task` ebenfalls anpassen
- [ ] Column-Struct um optionales `hidden`-Feld erweitern (für _archive-Spalte)
- [ ] `cargo build` ohne Errors und Warnings

## Technische Hinweise
- Datei: `src/main.rs`
- `#[serde(default)]` auf Struct-Level oder pro Feld
- Log-Format: "YYYY-MM-DD HH:MM moved from <spalte> to <spalte>"
- `column_id` wird in der Spec als `actual_row` bezeichnet – wir behalten `column_id` bei und fügen `previous_row` als neues Feld hinzu

## Dev Log
- Task-Struct erweitert um: `previous_row`, `points`, `worker`, `creator`, `logs`, `comments`
- `#[serde(default)]` auf Struct-Level + `impl Default for Task` für Rückwärtskompatibilität
- Column-Struct um `hidden: bool` mit `#[serde(default)]` erweitert
- `create_task` Handler: setzt `creator = "anonymous"` wenn leer
- `move_task` Handler: setzt `previous_row` auf alten `column_id`, schreibt Log-Eintrag mit Spaltennamen
- MCP `create_task`: nutzt `..Task::default()` für neue Felder, setzt `creator`
- MCP `move_task`: `previous_row` + Log-Eintrag analog zum REST-Handler
- `default_project()`: _archive-Spalte (order: 99, hidden: true) hinzugefügt
- `cargo build`: 0 errors, 0 warnings

## Tester Notes
- Alle 8 Anforderungen erfüllt
- Serde-Defaults korrekt implementiert (bestehende JSON-Dateien bleiben kompatibel)
- Log-Format korrekt: "YYYY-MM-DD HH:MM moved from X to Y"
- _archive-Spalte in default_project vorhanden
- Build: 0 errors, 0 warnings

## Abnahme
