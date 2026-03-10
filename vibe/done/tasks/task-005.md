# Task: Archive-Filterung in GET /api/projects/:id

**ID:** task-005
**Epic:** epic-002
**Status:** done
**Erstellt:** 2026-03-08
**Assignee:** developer

## Beschreibung
GET /api/projects/:id soll standardmäßig Tasks in der _archive-Spalte ausfiltern. Mit Query-Parameter `include_archived=true` werden alle Tasks zurückgegeben.

## Anforderungen
- [ ] Neuer Query-Parameter `include_archived` (bool, default false)
- [ ] Wenn false: Tasks mit column_id der _archive-Spalte aus Response filtern
- [ ] Wenn true: alle Tasks zurückgeben
- [ ] _archive-Spalte selbst wird bei hidden=true ebenfalls aus columns gefiltert (wenn nicht include_archived)

## Technische Hinweise
- `get_project` Handler anpassen
- Neuen Deserialize-Struct für Query-Parameter erstellen

## Dev Log
- `GetProjectQuery` Struct mit `include_archived: bool` (default false) erstellt
- `get_project` Handler: Query-Parameter extrahieren, hidden Spalten-IDs sammeln, Tasks + Columns filtern
- Bei `include_archived=true`: keine Filterung

## Tester Notes
- Code-Review: Filterlogik korrekt, sammelt hidden column IDs, filtert Tasks und Columns
- Build: 0 errors, 0 warnings
- Alle 4 Anforderungen erfüllt

## Abnahme
