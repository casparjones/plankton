# Task: Backend – Git-Service (clone, commit, push)

**ID:** task-043
**Epic:** epic-011
**Status:** done
**Erstellt:** 2026-03-08
**Assignee:** developer

## Anforderungen
- [x] Git-Service-Modul erstellen: `src/services/git_service.rs`
- [x] Funktion `sync_project_to_git(project, config)` – klont/öffnet Repo, schreibt Projektdatei, committed, pusht
- [x] Clone bei erstem Push, danach pull+push
- [x] Fehlerbehandlung: last_error in GitConfig setzen bei Fehler, last_push bei Erfolg
- [x] POST /api/projects/:id/git/sync Endpunkt für manuellen Sync
- [x] `cargo build` erfolgreich

## Umsetzung
- `src/services/git_service.rs`: open_or_clone, sync_project_to_git, perform_git_sync
- `src/controllers/git_controller.rs`: git_sync Handler + GitSyncResponse
- HTTPS-Token-Auth via URL-embedded Credentials
- Blocking-Thread für synchrone git2-Operationen
