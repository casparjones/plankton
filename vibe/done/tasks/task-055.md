# Task: Plankton-Tab UI mit Token-Eingabe und Preview

**ID:** task-055
**Epic:** epic-015
**Status:** done
**Erstellt:** 2026-03-14
**Assignee:** developer

## Beschreibung
Der "Plankton"-Tab im Prompt-Modal bekommt ein Formular zur Konfiguration
der MCP-Token-Informationen und eine Live-Preview der generierten Dateien.

## Anforderungen
- [x] Eingabefeld für Plankton-URL (Default: aktuelle Browser-URL)
- [x] Token-Bereich zeigt vorhandene Tokens mit Name, Rolle, Token-Wert, Status
- [x] Automatische Erstellung von drei Rollen-Tokens (Architect, Developer, Tester) wenn keine existieren
- [x] "Generieren"-Button erzeugt die drei Markdown-Dateien
- [x] Drei Ausgabe-Bereiche mit Sub-Tabs: secrets.md, rules.md, workflow.md
- [x] Hinweis dass Tokens unter Admin → Tokens verwaltet werden können
- [x] Konsistentes Design mit bestehendem Modal-Style

## Dev Log
- Token-Loading via `loadTokensForPrompt()` mit Auto-Erstellung
- Token-Liste mit XSS-geschütztem Rendering
- Nicht-Admin-User sehen Hinweis statt Token-Liste
- Backend-Fix: GET /api/admin/tokens gibt jetzt Token-Werte zurück

## Tester Notes
- Auto-Token-Erstellung getestet (3 Rollen: manager, developer, tester)
- XSS-Schutz via escapeHtml() verifiziert

## Abnahme
Alle Anforderungen erfüllt. Task abgenommen.
