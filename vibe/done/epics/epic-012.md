# Epic: Letztes Projekt im localStorage merken

**ID:** epic-012
**Status:** done
**Erstellt:** 2026-03-08
**Priorität:** high
**Quelle:** /flow/ideas/ux-improvements.md

## Beschreibung
Beim Projektwechsel wird die ID im localStorage gespeichert. Beim App-Start wird das zuletzt geöffnete Projekt automatisch geladen.

## Akzeptanzkriterien
- [ ] Beim Projektwechsel wird ID in localStorage gespeichert
- [ ] Beim App-Start wird gespeichertes Projekt geladen
- [ ] Fallback auf erstes Projekt wenn gespeichertes nicht mehr existiert
- [ ] Key enthält Username für Multi-User-Support

## Tasks
- [ ] task-037: localStorage-Logik in project-service.js und app.js implementieren
