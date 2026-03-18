# Task: Prompt-Modal auf Tab-System umbauen

**ID:** task-054
**Epic:** epic-015
**Status:** done
**Erstellt:** 2026-03-14
**Assignee:** developer

## Beschreibung
Das bestehende Prompt-Modal (`#prompt-modal`) wird um ein Tab-System erweitert.
Zwei Tabs: "Simple" (bisheriger Inhalt) und "Plankton" (neuer Tab).

## Anforderungen
- [x] Tab-Leiste im Modal-Header mit zwei Tabs: "Simple" und "Plankton"
- [x] Tab "Simple" zeigt den bisherigen Prompt-Inhalt (pre + Copy-Button) – keine Änderungen
- [x] Tab "Plankton" zeigt Konfiguration und Datei-Generator
- [x] Aktiver Tab ist visuell hervorgehoben
- [x] Beim Öffnen des Modals ist "Simple" der Default-Tab
- [x] Dark-Mode und Light-Mode kompatibel
- [x] Bestehende `generateProjectPrompt()` Funktionalität bleibt unangetastet

## Dev Log
- AppLayout.vue: Prompt-Modal HTML durch Tab-System ersetzt
- project-menu.ts: `initPromptTabs()` Funktion für Event-Listener-Registrierung
- project-menu.css: Neue Styles für `.prompt-tabs`, `.prompt-tab`, `.prompt-tab-content`

## Tester Notes
- Build erfolgreich (cargo + webpack)
- Tab-Wechsel korrekt implementiert via CSS-Klassen
- Simple-Tab funktional identisch mit vorheriger Version

## Abnahme
Alle Anforderungen erfüllt. Task abgenommen.
