# Task: Frontend – Locked-Column UI-Feedback

**ID:** task-041
**Epic:** epic-013
**Status:** done
**Erstellt:** 2026-03-08
**Assignee:** developer

## Anforderungen
- [x] Locked Columns: Lösch-Option im Kontextmenü deaktivieren
- [x] Tooltip: "Diese Spalte kann nicht gelöscht werden"
- [x] Frontend sendet `slug` und `locked` bei Column-Operationen mit
- [x] `npm run build` erfolgreich

## Umsetzung
- `src/frontend/components/column-modal.js`: Delete-Button im Kontextmenü erhält `col-ctx-disabled` + `disabled` + `title`-Tooltip wenn `col.locked === true`
- Click-Handler prüft zusätzlich `!col.locked` vor deleteColumn-Aufruf
- Column-Updates senden bereits alle Felder (inkl. slug/locked) via Spread-Operator
