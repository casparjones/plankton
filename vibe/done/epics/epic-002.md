# Epic: Datenmodell Task erweitern & Archiv-Logik

**ID:** epic-002
**Status:** done
**Erstellt:** 2026-03-08
**Priorität:** high

## Beschreibung
Das Task-Datenmodell auf die vollständige Spezifikation erweitern (points, worker, creator, logs, comments, previous_row). Archiv-Logik: Tasks die ≥14 Tage in "Done" liegen werden automatisch in eine versteckte `_archive`-Spalte verschoben.

## Akzeptanzkriterien
- [x] Task-Struct enthält alle neuen Felder: previous_row, points, worker, creator, logs, comments
- [x] Bestehende API-Handler arbeiten korrekt mit dem erweiterten Modell
- [x] Background-Task (tokio::spawn) prüft alle 24h und archiviert fällige Tasks
- [x] GET /api/projects/:id filtert _archive-Tasks standardmäßig aus
- [x] GET /api/projects/:id?include_archived=true gibt alle Tasks inkl. Archiv zurück
- [x] Beim Archivieren wird Log-Eintrag angehängt

## Tasks
- [x] task-003: Task-Struct erweitern + API-Handler anpassen
- [x] task-004: Archiv-Logik mit Background-Task implementieren
- [x] task-005: Archive-Filterung in GET /api/projects/:id

## Notizen
- Serde-Defaults verwenden für Rückwärtskompatibilität mit bestehenden JSON-Dateien
- Column-Struct braucht evtl. ein `hidden`-Feld
