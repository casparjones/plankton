# Task: Letztes Projekt im localStorage merken

**ID:** task-037
**Epic:** epic-012
**Status:** done
**Erstellt:** 2026-03-08
**Assignee:** developer

## Beschreibung
Beim Projektwechsel und Projekt-Erstellung die ID im localStorage speichern.
Beim App-Start das gespeicherte Projekt laden.

## Anforderungen
- [ ] localStorage Key: `plankton_last_project_<username>`
- [ ] In `openProject()` und `createProject()`: ID speichern
- [ ] In `startApp()`: gespeichertes Projekt laden (Fallback: erstes)
- [ ] Wenn gespeichertes Projekt nicht existiert: Fallback + bereinigen
- [ ] `npm run build` erfolgreich
