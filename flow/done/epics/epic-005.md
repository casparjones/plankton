# Epic: Frontend Build-Integration & Developer Experience

**ID:** epic-005
**Status:** done
**Erstellt:** 2026-03-08
**Priorität:** low

## Beschreibung
Reibungsloser Entwicklungs-Workflow sicherstellen: cargo run baut Frontend automatisch, npm run dev für Watch-Modus, .gitignore korrekt, README mit Setup-Anleitung.

## Akzeptanzkriterien
- [x] `cargo run` baut automatisch das Frontend (build.rs funktioniert korrekt)
- [x] `npm run dev` startet Webpack im Watch-Modus (package.json korrekt)
- [x] .gitignore enthält: target/, node_modules/, static/bundle.js, static/bundle.css, data/
- [ ] README.md mit vollständiger Setup-Anleitung (nicht erstellt – readme nur auf explizite Anfrage)

## Tasks
- [x] task-013: build.rs geprüft (OK) + .gitignore vervollständigt
- [ ] task-014: README.md (offen – wird nur auf explizite Anfrage erstellt)

## Notizen
- build.rs sollte bereits vorhanden sein – Funktionalität verifizieren
- Webpack-Config prüfen ob alles korrekt konfiguriert ist
