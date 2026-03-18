# Epic: KI-Agenten-Workflow Prompt-Generator

**ID:** epic-015
**Status:** done
**Erstellt:** 2026-03-14
**Priorität:** high

## Beschreibung
Das Prompt-Modal wurde um ein Tab-System erweitert. Tab 1 ("Simple") ist der bisherige Prompt
für schnelle Task-Erstellung als JSON. Tab 2 ("Plankton") generiert drei Markdown-Dateien
(secrets.md, rules.md, workflow.md), die den vollständigen KI-Agenten-Workflow für Claude Code
konfigurieren.

## Akzeptanzkriterien
- [x] Prompt-Modal hat zwei Tabs: "Simple" und "Plankton"
- [x] Tab "Simple" funktioniert exakt wie bisher
- [x] Tab "Plankton" zeigt Formular für MCP-Token-Konfiguration
- [x] Drei Markdown-Dateien werden generiert: secrets.md, rules.md, workflow.md
- [x] Jede Datei hat einen eigenen Copy-Button und Download-Button
- [x] secrets.md enthält alle konfigurierten MCP-Tokens
- [x] rules.md beschreibt Plankton und den Agenten-Prompt
- [x] workflow.md beschreibt den Agenten-Workflow (ohne Secrets)
- [x] UI ist konsistent mit dem bestehenden Design (Dark/Light Mode)
- [x] Keine Breaking Changes an bestehender Funktionalität
- [x] Automatische Token-Erstellung (Architect, Developer, Tester) wenn keine existieren
- [x] Hinweis auf Token-Verwaltung in Admin-Oberfläche

## Tasks
- [x] task-054: Prompt-Modal auf Tab-System umbauen
- [x] task-055: Plankton-Tab UI mit Token-Eingabe und Preview
- [x] task-056: Markdown-Generator für die drei Konfigurationsdateien
- [x] task-057: Download- und Copy-Funktionalität für generierte Dateien

## Notizen
- Backend-Änderung: GET /api/admin/tokens gibt jetzt Token-Werte zurück (für Admin-User)
- Neue Datei: src/frontend/components/prompt-generator.ts
