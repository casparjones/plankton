# Task-021: localStorage-Persistierung & System-Präferenz

**Epic:** epic-007
**Status:** open
**Rolle:** Developer

## Beschreibung
Theme-Auswahl im localStorage speichern. Beim Laden der App gespeichertes Theme anwenden. Fallback: System-Präferenz (prefers-color-scheme), dann Dark Mode.

## Akzeptanzkriterien
- [ ] Theme-Wahl wird in localStorage unter "plankton-theme" gespeichert
- [ ] Beim App-Start: localStorage prüfen → System-Präferenz → Fallback Dark
- [ ] Nach Browser-Reload bleibt das gewählte Theme erhalten
- [ ] Initiales Theme wird vor DOMContentLoaded gesetzt (kein Flash)
