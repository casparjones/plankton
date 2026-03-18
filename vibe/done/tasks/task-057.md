# Task: Download- und Copy-Funktionalität für generierte Dateien

**ID:** task-057
**Epic:** epic-015
**Status:** done
**Erstellt:** 2026-03-14
**Assignee:** developer

## Beschreibung
Jede generierte Markdown-Datei bekommt einen Copy-to-Clipboard-Button
und einen Download-Button.

## Anforderungen
- [x] Copy-Button kopiert Inhalt der aktuell sichtbaren Datei in die Zwischenablage
- [x] Download-Button lädt die aktuell sichtbare Datei als .md herunter
- [x] Visuelles Feedback beim Kopieren (Button-Text ändert sich kurz zu "✓ Kopiert")
- [x] Download-Dateiname entspricht dem Dateinamen (secrets.md, rules.md, workflow.md)
- [x] Buttons sind kontextabhängig zum aktiven Output-Tab

## Dev Log
- `copyToClipboard()` mit visuellem Feedback (1.5s Timeout)
- `downloadFile()` via Blob + URL.createObjectURL
- Beide Funktionen arbeiten mit dem aktiven Output-Tab (`activeOutputTab`)

## Tester Notes
- Copy und Download korrekt implementiert
- Feedback-Timer funktioniert

## Abnahme
Alle Anforderungen erfüllt. Task abgenommen.
