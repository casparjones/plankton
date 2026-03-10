# Task: Frontend aufteilen – API, State, Services, Komponenten

**ID:** task-033
**Epic:** epic-010
**Status:** done
**Erstellt:** 2026-03-08
**Assignee:** developer

## Beschreibung
Die monolithische static/main.js (2019 Zeilen) in modulare ES-Module unter src/frontend/ aufteilen.
Webpack Entry-Point auf src/frontend/main.js umstellen.

## Anforderungen
- [ ] src/frontend/api.js – API-Client (get, post, put, del)
- [ ] src/frontend/state.js – Zentraler State + COLUMN_COLORS
- [ ] src/frontend/services/ – project-service.js, task-service.js, sse-service.js, column-service.js
- [ ] src/frontend/components/ – board.js, sidebar.js, task-modal.js, task-detail.js, theme.js, auth.js, admin.js, column-modal.js, prompt-modal.js, password-modal.js, json-view.js, bulk-actions.js
- [ ] src/frontend/utils.js – escapeHtml, formatDate, columnName
- [ ] src/frontend/dom.js – buildDOM
- [ ] src/frontend/main.js – init() + DOMContentLoaded
- [ ] webpack.config.js: Entry auf src/frontend/main.js
- [ ] build.rs: rerun-if-changed auf src/frontend/
- [ ] CSS als import './styles.css' beibehalten (Pfad anpassen)
- [ ] `npm run build` erfolgreich
- [ ] Keine UI-Änderungen

## Technische Hinweise
- Alle Module als ES-Module mit import/export
- state als Singleton-Objekt das von allen Modulen importiert wird
- Zirkuläre Imports vermeiden
