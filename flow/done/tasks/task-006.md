# Task: Projekt inline editieren + Projekt löschen + _archive Default

**ID:** task-006
**Epic:** epic-003
**Status:** done
**Erstellt:** 2026-03-08
**Assignee:** developer

## Beschreibung
Frontend-Erweiterungen für vollständiges Projekt-Management: Projektnamen per Doppelklick in der Sidebar editieren, Projekte löschen mit Bestätigungs-Dialog, und Frontend-seitiges createProject() um _archive-Spalte erweitern.

## Anforderungen
- [ ] Doppelklick auf Projektname in Sidebar → inline Input-Feld, Enter/Blur speichert
- [ ] Löschen-Button (X oder Mülleimer-Icon) pro Projekt in Sidebar
- [ ] Bestätigungs-Dialog: "Projekt 'X' und alle Tasks wirklich löschen?"
- [ ] Bei letztem Projekt: Löschen-Button deaktiviert/versteckt
- [ ] Frontend `createProject()`: _archive-Spalte mit `hidden: true` hinzufügen
- [ ] Versteckte Spalten (_archive) werden im Board NICHT angezeigt (columns filtern)
- [ ] `cargo build` nicht nötig (reine Frontend-Änderung)

## Technische Hinweise
- Datei: `static/main.js`
- `api.del()` braucht rev-Parameter → muss aus dem Projekt-Objekt geholt werden
- DELETE endpoint: `/api/projects/:id?rev=:rev`
- PUT für Umbenennen: ganzes Projekt-Objekt senden

## Dev Log
- `renameProject()` und `deleteProject()` Funktionen hinzugefügt
- `renderProjectList()` komplett überarbeitet: Span für Name + Doppelklick-Edit + Löschen-Button
- Inline-Edit: Input ersetzt Span, Enter/Blur speichert, Escape bricht ab
- Löschen-Button nur bei >1 Projekt sichtbar, mit confirm-Dialog
- `createProject()` um _archive-Spalte erweitert
- `renderBoard()` filtert `hidden` Spalten aus
- CSS: .project-name, .project-delete-btn (opacity-Transition), .project-rename-input

## Tester Notes
- Alle 7 Anforderungen erfüllt
- Doppelklick-Edit korrekt implementiert (Enter, Blur, Escape)
- Löschen korrekt mit Rev-Parameter und confirm
- _archive-Spalte korrekt ausgeblendet
- Backend Build: 0 errors, 0 warnings

## Abnahme
