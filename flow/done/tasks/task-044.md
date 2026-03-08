# Task: Backend – Auto-Push nach Projektänderungen

**ID:** task-044
**Epic:** epic-011
**Status:** done
**Erstellt:** 2026-03-08
**Assignee:** developer

## Anforderungen
- [x] Nach jeder Projektänderung wird automatisch ein Git-Sync getriggert
- [x] Async und non-blocking: tokio::spawn, kein Warten auf das Ergebnis
- [x] Nur wenn git.enabled == true
- [x] `cargo build` erfolgreich

## Umsetzung
- `publish_update()` ruft `trigger_git_sync()` auf (fire-and-forget)
- `trigger_git_sync()` prüft ob Git aktiviert ist, spawnt dann `perform_git_sync`
