# Task: Frontend Komponenten aus app.js extrahieren

**ID:** task-034
**Epic:** epic-010
**Status:** done
**Erstellt:** 2026-03-08
**Assignee:** developer

## Beschreibung
Die app.js (~2000 Zeilen) in separate Module aufteilen. Jede logische Einheit bekommt ein eigenes Modul.

## Anforderungen
- [ ] src/frontend/components/theme.js – applyTheme, toggleTheme, initTheme
- [ ] src/frontend/components/auth.js – checkAuth, doLogin, doLogout, renderLoginView
- [ ] src/frontend/components/sidebar.js – renderSidebar, Projekt-CRUD UI
- [ ] src/frontend/components/board.js – renderBoard, initKanban, refreshBoard
- [ ] src/frontend/components/task-modal.js – openTaskModal, saveTask
- [ ] src/frontend/components/task-detail.js – showTaskDetail, renderDetailView
- [ ] src/frontend/components/column-modal.js – openColumnModal
- [ ] src/frontend/components/admin.js – renderAdminPanel
- [ ] src/frontend/components/bulk-actions.js – Bulk-Task-Operationen
- [ ] src/frontend/services/sse-service.js – SSE-Verbindung
- [ ] src/frontend/services/project-service.js – loadProject, loadProjects
- [ ] src/frontend/dom.js – buildDOM Funktion
- [ ] app.js nur noch init() + Orchestrierung
- [ ] `npm run build` erfolgreich
- [ ] Keine UI-Änderungen

## Technische Hinweise
- Alle Module als ES-Module mit import/export
- state wird von allen Modulen aus state.js importiert
- Zirkuläre Imports vermeiden: Callbacks/Events statt direkte Cross-Imports
- Schrittweise vorgehen: ein Modul nach dem anderen extrahieren und nach jedem Schritt bauen
