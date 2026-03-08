# Task: Backend – Column-Slug und locked-Feld

**ID:** task-038
**Epic:** epic-013
**Status:** done
**Erstellt:** 2026-03-08
**Assignee:** developer

## Beschreibung
Column-Struct um `slug` (String) und `locked` (bool) erweitern.
Slug wird automatisch aus dem Titel normalisiert.
Todo und _archive sind locked.

## Anforderungen
- [ ] Column-Struct: `slug: String` und `locked: bool` Felder hinzufügen
- [ ] Slug-Normalisierung: Trim, Sonderzeichen entfernen, Leerzeichen→Unterstrich, Uppercase
- [ ] Default-Projekt: Todo→`TODO`, In Progress→`IN_PROGRESS`, Done→`DONE`, _archive→`_ARCHIVE`
- [ ] Todo und _archive: locked=true
- [ ] Bei Column-Erstellung/Update: Slug automatisch generieren
- [ ] Slug muss pro Projekt eindeutig sein
- [ ] Locked Columns können nicht gelöscht werden (API-Schutz)
- [ ] Tasks können per `column_slug` statt `column_id` erstellt werden
- [ ] `cargo build` erfolgreich
