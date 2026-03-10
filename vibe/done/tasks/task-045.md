# Task: Frontend – Git-Tab in Projekt-Einstellungen

**ID:** task-045
**Epic:** epic-011
**Status:** done
**Erstellt:** 2026-03-08
**Assignee:** developer

## Anforderungen
- [x] Neues Modul `src/frontend/components/git-settings.js`
- [x] Git-Button im Projekt-Dropdown
- [x] Felder: Repo-URL, Branch, Pfad, Enabled-Toggle
- [x] Status-Anzeige: letzter Push-Zeitpunkt, letzter Fehler
- [x] Button "Jetzt synchronisieren"
- [x] CSS für Git-Einstellungen
- [x] `npm run build` erfolgreich

## Umsetzung
- `src/frontend/components/git-settings.js`: openGitModal, closeGitModal, saveGitConfig, triggerGitSync
- `src/frontend/components/project-menu.js`: Git-Button im Dropdown
- `src/frontend/dom.js`: Git-Modal HTML + Event-Listener
- `static/styles/git.css`: Styling
