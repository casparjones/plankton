# Epic: Backend Refactoring – main.rs aufteilen

**ID:** epic-009
**Status:** done
**Erstellt:** 2026-03-08
**Priorität:** high
**Quelle:** /flow/ideas/refactoring.md

## Beschreibung
Die main.rs (2609 Zeilen) enthält alles: Routing, Handler, Business-Logik, Datenmodelle, Store-Implementierungen.
Aufteilen in eine saubere Modulstruktur gemäß der Idee in refactoring.md.

## Akzeptanzkriterien
- [ ] main.rs enthält nur noch Router-Aufbau und Server-Start (~50-100 Zeilen)
- [ ] Models in src/models/ (project.rs, requests.rs)
- [ ] Controller in src/controllers/ (project, task, column, user, event, mcp, auth)
- [ ] Services in src/services/
- [ ] Store in src/store/ (couch.rs, file.rs)
- [ ] config.rs und error.rs und state.rs als eigene Module
- [ ] `cargo build` läuft fehlerfrei
- [ ] Keine API-Änderungen – alle Endpunkte bleiben identisch
- [ ] Keine neuen Warnings

## Tasks
- [ ] task-028: Models extrahieren (project.rs, requests.rs)
- [ ] task-029: Store extrahieren (mod.rs, couch.rs, file.rs)
- [ ] task-030: State, Config, Error als eigene Module
- [ ] task-031: Controller extrahieren (project, task, column, user, event, mcp, auth)
- [ ] task-032: Services extrahieren + main.rs aufräumen

## Notizen
- Reihenfolge ist wichtig: erst Models, dann Store, dann State/Config/Error, dann Controller, dann Services
- Nach jedem Task muss `cargo build` erfolgreich sein
- Keine funktionalen Änderungen, nur Struktur
