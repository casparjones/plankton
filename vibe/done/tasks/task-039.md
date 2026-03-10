# Task: Backend – Import-Endpunkt mit Validierung

**ID:** task-039
**Epic:** epic-013
**Status:** done
**Erstellt:** 2026-03-08
**Assignee:** developer

## Beschreibung
POST /api/projects/:id/import Endpunkt der eine Liste von Tasks validiert und importiert.

## Anforderungen
- [ ] ImportRequest: { tasks: Vec<Task> }
- [ ] ImportResponse: { imported, warnings, errors, skipped }
- [ ] Validierung: title pflicht, column_slug auflösen, points 0-100
- [ ] Fehlende optionale Felder automatisch setzen
- [ ] Log-Eintrag pro importiertem Task
- [ ] Route registrieren
- [ ] `cargo build` erfolgreich
