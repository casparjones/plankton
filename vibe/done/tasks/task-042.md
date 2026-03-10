# Task: Backend – Git-Datenmodell & Konfiguration

**ID:** task-042
**Epic:** epic-011
**Status:** done
**Erstellt:** 2026-03-08
**Assignee:** developer

## Anforderungen
- [x] `git2` Crate zu Cargo.toml hinzufügen
- [x] `GitConfig` Struct erstellen: `repo_url`, `branch`, `path`, `enabled`, `last_push`, `last_error`
- [x] `GitConfig` als optionales Feld in `Project` Struct einbetten
- [x] API-Endpunkte: PUT /api/projects/:id/git (Konfiguration setzen), GET /api/projects/:id/git (Konfiguration lesen)
- [x] `cargo build` erfolgreich

## Umsetzung
- `src/models/project.rs`: GitConfig Struct mit serde defaults
- `src/controllers/git_controller.rs`: GET/PUT Handler
- `src/main.rs`: Route `/api/projects/:id/git` registriert
- `src/services/project_service.rs`: `git: None` im default_project
