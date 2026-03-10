# Task: Controller extrahieren

**ID:** task-031
**Epic:** epic-009
**Status:** done
**Erstellt:** 2026-03-08
**Assignee:** developer

## Beschreibung
Alle Handler-Funktionen aus main.rs in src/controllers/ extrahieren.

## Anforderungen
- [ ] src/controllers/mod.rs
- [ ] src/controllers/project_controller.rs (list, create, get, update, delete)
- [ ] src/controllers/task_controller.rs (create, update, delete, move)
- [ ] src/controllers/column_controller.rs (create, update, delete)
- [ ] src/controllers/user_controller.rs (create, update, delete)
- [ ] src/controllers/event_controller.rs (SSE project_events)
- [ ] src/controllers/mcp_controller.rs (list_tools, call_tool, mcp_jsonrpc, execute_tool, docs_page)
- [ ] src/controllers/auth_controller.rs (login, logout, me, change_password)
- [ ] src/controllers/admin_controller.rs (admin user + token CRUD)
- [ ] `cargo build` erfolgreich

## Technische Hinweise
- Handler nutzen AppState, ApiError, Models
- publish_update und default_project werden als Hilfsfunktionen gebraucht
- auth_guard Middleware muss auch extrahiert werden
- request_logger und print_startup_banner können in main bleiben oder in ein middleware-Modul
