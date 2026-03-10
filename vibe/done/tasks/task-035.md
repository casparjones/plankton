# Task: CSS aufteilen in styles/ Unterverzeichnis

**ID:** task-035
**Epic:** epic-010
**Status:** done
**Erstellt:** 2026-03-08
**Assignee:** developer

## Beschreibung
Die monolithische static/styles.css in thematische Untermodule aufteilen und per CSS @import zusammenführen.

## Anforderungen
- [ ] static/styles/ Verzeichnis erstellen
- [ ] CSS in logische Module aufteilen (base, layout, sidebar, board, modals, admin, theme, etc.)
- [ ] static/styles.css als Hauptdatei die Module importiert
- [ ] Webpack CSS-Import bleibt funktionsfähig
- [ ] `npm run build` erfolgreich
- [ ] Keine visuellen Änderungen

## Technische Hinweise
- CSS @import oder Webpack-kompatible Imports nutzen
- Variablen (CSS Custom Properties) in eigene Datei
