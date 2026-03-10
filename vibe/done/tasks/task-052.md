# Task: Migration Board-Komponente – jKanban → VueDraggablePlus

**ID:** task-052
**Epic:** epic-014
**Status:** done
**Erstellt:** 2026-03-10
**Assignee:** developer

## Beschreibung
Die bestehende jKanban-basierte Board-Komponente durch eine native Vue.js 3 Implementierung mit **VueDraggablePlus** ersetzen. jKanban und zugehörige Webpack-Hacks werden komplett entfernt.

## Anforderungen
- [x] `vue-draggable-plus` als Dependency in package.json hinzufügen
- [x] `jkanban` aus package.json entfernen
- [x] `exports-loader`, `string-replace-loader`, `imports-loader` aus devDependencies entfernen
- [x] jKanban-Webpack-Rule aus webpack.config.js entfernen
- [x] `components/KanbanBoard.vue` erstellen: Vollständige Board-Komponente mit VueDraggablePlus
- [x] Spalten als horizontale Container
- [x] Tasks innerhalb jeder Spalte per Drag&Drop verschiebbar (zwischen Spalten und innerhalb)
- [x] Beim Verschieben eines Tasks: API-Call `POST /api/projects/:id/tasks/:task_id/move` auslösen
- [x] Task-Karten anzeigen: Titel, Beschreibung (erste 80 Zeichen), Points-Badge, Worker-Initial, Labels
- [x] Spalten-Header: Name + Anzahl Tasks + Farbstreifen + Add-Button + Menu-Button
- [x] Leere Spalten zeigen "Keine Tasks" Placeholder
- [x] Versteckte Spalten (`hidden: true`) werden nicht angezeigt
- [x] Done-Spalte wird immer zuletzt sortiert
- [x] AppLayout.vue anpassen: Board-Container nutzt KanbanBoard.vue
- [x] `npm run build` fehlerfrei
- [x] `cargo build` fehlerfrei

## Technische Hinweise
- VueDraggablePlus: `npm install vue-draggable-plus` – nutzt `<VueDraggable>` Komponente
- Bestehende board.js (95 Zeilen) als Referenz für die Logik
- Bestehende task-modal.js `taskToItem()` Funktion als Referenz für Task-Karten-Rendering
- API für Task-Move: `POST /api/projects/:id/tasks/:task_id/move` mit Body `{ column_id, order }`
- SSE-Service muss weiterhin Board-Updates triggern können
- state.isDragging Flag setzen während Drag-Operationen

## Dev Log
- `vue-draggable-plus` installiert, `jkanban` + `exports-loader` + `string-replace-loader` + `imports-loader` deinstalliert
- jKanban-Webpack-Rule komplett entfernt aus webpack.config.js
- `KanbanBoard.vue`: Neue Vue-Komponente mit VueDraggable, computed columns, reactive task-lists
- Task-Karten als Vue-Template mit v-for, v-if, :style Bindings (kein innerHTML mehr)
- Drag&Drop: VueDraggable group="tasks", onDragStart/onDragEnd für isDragging-Flag
- Task-Move: onTaskChange() ruft moveTask() aus project-service.js auf
- Worker-Farbzuweisung: Hash-basierte Farbpalette (identisch mit vorheriger Implementierung)
- `board.js` zu Bridge-Modul umgebaut: renderBoard() ruft window.__kanbanRefresh()
- AppLayout.vue: `<KanbanBoard />` Komponente im Board-Container, handleBoardClick entfernt
- Bundle: 166 KiB (vorher 139 KiB – VueDraggablePlus ist größer aber wartbarer als jKanban)
- Build: npm + cargo fehlerfrei

## Tester Notes
- Build fehlerfrei
- jKanban komplett entfernt, keine Referenzen mehr
- VueDraggablePlus korrekt integriert
- Task-Karten-Rendering als Vue-Template (typisiert, kein innerHTML)
- Legacy-Kompatibilität: board.js Bridge funktioniert für SSE und andere Module

## Abnahme
