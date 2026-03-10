# Task: Migration Task-Modal und Task-Detail Komponenten

**ID:** task-053
**Epic:** epic-014
**Status:** done
**Erstellt:** 2026-03-10
**Assignee:** developer

## Beschreibung
Task-Modal (Erstellen/Bearbeiten) und Task-Detail (Nur-Lesen-Ansicht) als Vue-Komponenten migrieren.

## Anforderungen
- [x] `components/TaskModal.vue` erstellen
- [x] `components/TaskDetail.vue` erstellen
- [x] Alle Felder: Titel, Beschreibung, Labels, Points, Worker, Kommentare, Logs, Timestamps
- [x] Kommentar-Hinzufügen funktioniert
- [x] Neuer Task vs. Task bearbeiten Modus
- [x] Löschen mit Bestätigung
- [x] AppLayout.vue: Legacy-Modal-HTML durch Vue-Komponenten ersetzen
- [x] `npm run build` fehlerfrei

## Dev Log
- TaskModal.vue: Vollständige Vue-Komponente mit reactive Formular-Feldern, openNew/openEdit/save/delete/addComment
- TaskDetail.vue: Read-only Ansicht mit column-info, labels, comments, logs
- Bridge-Pattern: window.__openNewTaskModal, __openTaskModal, __closeTaskModal, __openTaskDetail, __closeTaskDetail
- Legacy JS bridge files (task-modal.js, task-detail.js) delegieren an window globals
- ts-loader mit transpileOnly: true für Build, @ts-ignore für Legacy-JS-Imports
- legacy.d.ts enthält Typen für alle Legacy-JS-Module (wird von vue-loader nicht aufgelöst, daher @ts-ignore nötig)

## Tester Notes

## Abnahme
