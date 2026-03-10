# Epic: Frontend Refactoring – main.js aufteilen

**ID:** epic-010
**Status:** done
**Erstellt:** 2026-03-08
**Priorität:** medium
**Quelle:** /flow/ideas/refactoring.md

## Beschreibung
Die static/main.js (2019 Zeilen) ist monolithisch. Aufteilen in modulare Komponenten unter src/frontend/.
Webpack Entry-Point auf src/frontend/main.js umstellen.

## Akzeptanzkriterien
- [ ] src/frontend/main.js als neuer Entry-Point
- [ ] Komponenten: board.js, task-card.js, task-modal.js, sidebar.js
- [ ] Services: api.js, state.js, project-service.js, task-service.js, sse-service.js
- [ ] CSS aufgeteilt in styles/ Unterverzeichnis
- [ ] webpack.config.js aktualisiert
- [ ] `npm run build` läuft fehlerfrei
- [ ] Keine UI-Änderungen – Board sieht identisch aus

## Tasks
- [x] task-033: Frontend API-Modul und State-Modul extrahieren (done – Grundstruktur steht)
- [x] task-034: Frontend Komponenten extrahieren (done – 20 Module)
- [x] task-035: CSS aufteilen und Webpack-Config anpassen (done – 10 Module)
- [x] task-036: Dockerfile anpassen (done – src/frontend/ + static/styles/ Pfade)


## Notizen
- Abhängig von Epic 009 (Backend sollte zuerst fertig sein, damit Testbasis stabil ist)
- build.rs muss angepasst werden (rerun-if-changed Pfade)
