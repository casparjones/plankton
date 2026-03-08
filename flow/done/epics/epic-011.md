# Epic: Git-Repository-Integration für Projekte

**ID:** epic-011
**Status:** done
**Erstellt:** 2026-03-08
**Priorität:** low
**Quelle:** /flow/ideas/git.md

## Beschreibung
Jedes Plankton-Projekt kann optional mit einem Git-Repository verknüpft werden.
Bei Board-Änderungen wird automatisch ein Commit/Push in das hinterlegte Repo durchgeführt.
Vollständige Historie aller Board-Änderungen direkt in Git.

## Akzeptanzkriterien
- [x] Git-Konfiguration pro Projekt (Repo-URL, Branch, Pfad, Auth)
- [x] Auto-Push nach jeder Projektänderung (async, nicht-blockierend)
- [x] Push-Status wird im Projekt gespeichert
- [x] Manueller Sync/Pull auf Knopfdruck
- [x] Frontend: Git-Tab in Projekteinstellungen
- [x] Frontend: Git-Status-Icon im Board-Header
- [x] Konflikt-Behandlung (hard reset bei Pull, commit+push bei Änderungen)

## Tasks
- [x] task-042: Backend – Git-Datenmodell & Konfiguration (GitConfig im Project-Struct)
- [x] task-043: Backend – Git-Service (clone, commit, push via git2)
- [x] task-044: Backend – Auto-Push nach Projektänderungen (async, non-blocking)
- [x] task-045: Frontend – Git-Tab in Projekt-Einstellungen
- [x] task-046: Frontend – Git-Status-Icon im Board-Header + Manual Sync
