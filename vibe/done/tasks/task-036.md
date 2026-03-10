# Task: Dockerfile anpassen für neuen Frontend-Build

**ID:** task-036
**Epic:** epic-010
**Status:** done
**Erstellt:** 2026-03-08
**Assignee:** developer

## Beschreibung
Das Dockerfile muss den neuen src/frontend/ Pfad korrekt berücksichtigen, damit der Docker-Build weiterhin funktioniert.

## Anforderungen
- [ ] Dockerfile prüfen: src/frontend/ wird in Stage 1 (Node) korrekt kopiert
- [ ] Webpack-Build im Docker-Container funktioniert
- [ ] Rust-Build in Stage 2 findet bundle.js/bundle.css
- [ ] Docker build erfolgreich

## Technische Hinweise
- build.rs erkennt vorhandene bundle.js und überspringt npm
- Stage 1 braucht: package.json, webpack.config.js, src/frontend/, static/styles.css
