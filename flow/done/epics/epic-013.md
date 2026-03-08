# Epic: Row-Slugs & KI Issue-Import

**ID:** epic-013
**Status:** done
**Erstellt:** 2026-03-08
**Priorität:** medium
**Quelle:** /flow/ideas/json-structure.md

## Beschreibung
Spalten bekommen normalisierte Slugs (TODO, IN_PROGRESS, DONE) statt nur UUIDs.
Ein Import-Dialog ermöglicht es, KI-generierte Task-Listen als JSON zu importieren.

## Akzeptanzkriterien
- [x] Column-Struct bekommt `slug`- und `locked`-Feld
- [x] Slug wird automatisch aus Titel generiert
- [x] Tasks können per `column_slug` statt `column_id` referenziert werden
- [x] Import-Endpunkt: POST /api/projects/:id/import
- [x] Frontend: Import-Dialog mit Validierung und Preview
- [x] Locked-Spalten (TODO, _ARCHIVE) können nicht gelöscht werden

## Tasks
- [x] task-038: Backend – Column-Slug und locked-Feld zum Datenmodell hinzufügen
- [x] task-039: Backend – Import-Endpunkt mit Validierung
- [x] task-040: Frontend – Import-Dialog Modal
- [x] task-041: Frontend – Locked-Column UI-Feedback
