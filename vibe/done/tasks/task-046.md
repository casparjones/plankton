# Task: Frontend – Git-Status-Icon im Board-Header

**ID:** task-046
**Epic:** epic-011
**Status:** done
**Erstellt:** 2026-03-08
**Assignee:** developer

## Anforderungen
- [x] Git-Status-Icon neben dem Projektnamen im Board-Header
- [x] Icon-Zustände: kein Git (versteckt), aktiviert+OK (grün), Fehler (rot), deaktiviert (grau)
- [x] Klick auf Icon öffnet Git-Modal
- [x] Icon aktualisiert sich bei renderBoard
- [x] `npm run build` erfolgreich

## Umsetzung
- `src/frontend/components/git-settings.js`: updateGitStatusIcon()
- `src/frontend/components/board.js`: ruft updateGitStatusIcon() bei renderBoard auf
- `src/frontend/dom.js`: Icon-Element + Click-Listener
- `static/styles/git.css`: Icon-Styles mit Farbzuständen + Puls-Animation bei Fehler
