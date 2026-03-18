# Task: Markdown-Generator für die drei Konfigurationsdateien

**ID:** task-056
**Epic:** epic-015
**Status:** done
**Erstellt:** 2026-03-14
**Assignee:** developer

## Beschreibung
Implementierung der Generierungslogik für die drei Markdown-Dateien.

## Anforderungen
- [x] `generateSecretsMd(tokens, url)`: Erzeugt secrets.md mit Token-Tabelle
- [x] `generateRulesMd(url, projectName)`: Erzeugt rules.md mit MCP-Tools und Agenten-Hierarchie
- [x] `generateWorkflowMd()`: Erzeugt workflow.md mit vollständigem Workflow
- [x] secrets.md enthält Warnung dass die Datei nicht ins Git darf
- [x] rules.md enthält die 4-Agenten-Hierarchie (Supervisor, Architect, Developer, Tester)
- [x] workflow.md beschreibt den vollständigen Workflow (Idee→Epic→Tasks→Review→Done)
- [x] rules.md enthält die Plankton-URL und Dokumentationslink

## Dev Log
- Neue Datei: `src/frontend/components/prompt-generator.ts`
- Drei exportierte Funktionen als reine String-Templates
- TokenEntry-Interface für typsichere Token-Übergabe

## Tester Notes
- Alle drei Generatoren produzieren valides Markdown
- Keine DOM-Abhängigkeiten in prompt-generator.ts

## Abnahme
Alle Anforderungen erfüllt. Task abgenommen.
