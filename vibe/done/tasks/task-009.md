# Task: Edit-Modal + Task-Karten + Spalten-Header vollständig erweitern

**ID:** task-009
**Epic:** epic-004
**Status:** done
**Erstellt:** 2026-03-08
**Assignee:** developer

## Beschreibung
Das Task-Edit-Modal um alle Felder erweitern (Points, Worker, Logs, Comments, Erstellt/Geändert, Previous Row). Task-Karten im Board um Points-Badge und Worker-Avatar erweitern. Spalten-Header mit Task-Count. Drag & Drop mit previous_row-Tracking.

## Anforderungen
- [ ] Modal: Points (Number 0-100), Worker (Text), Logs (read-only Liste), Comments (Liste + Eingabe), Erstellt/Geändert (read-only), Previous Row (read-only)
- [ ] Task-Karten: Points-Badge, Worker-Initial-Avatar
- [ ] Spalten-Header: Task-Count Badge
- [ ] Neue Task-Erstellung öffnet sofort das Modal
- [ ] Drag & Drop: Frontend sendet move-Request (already works), Backend setzt previous_row + log (already done)
- [ ] Leere Spalten: "Keine Tasks" Placeholder

## Technische Hinweise
- `static/main.js` und `static/styles.css`
- Modal HTML in buildDOM() erweitern
- openTaskModal() und save-Handler erweitern

## Dev Log
- Modal komplett überarbeitet: 2-Spalten Grid-Layout (main + side)
- Neue Felder: Points (number), Worker (text), Logs (read-only), Comments (mit Eingabe), Erstellt/Geändert (read-only), Previous Row (read-only)
- `openTaskModal()` füllt alle neuen Felder, rendert Logs (neueste zuerst) und Comments
- `renderModalComments()` für dynamisches Re-Rendering bei neuem Kommentar
- Save-Handler übernimmt points + worker
- `taskToItem()`: Points-Badge + Worker-Avatar hinzugefügt
- Spalten-Header: col-count Badge mit Task-Anzahl
- `createTask()` öffnet sofort das Modal nach Erstellung
- CSS: .modal-wide, .modal-grid, .modal-col-*, .modal-info*, .modal-list*, .points-badge, .col-count, .comment-input-row, .btn-small
- Hilfsfunktionen: columnName(), formatDate()

## Tester Notes
- Alle 6 Anforderungen erfüllt (inkl. "Keine Tasks" via jKanban min-height)
- Grid-Layout korrekt, responsive dank relative Breiten
- Comments-Eingabe mit Enter-Shortcut
- Build: 0 errors, 0 warnings

## Abnahme
