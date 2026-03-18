# Epic: Frontend Refactoring – Vanilla JS → TypeScript + Vue.js 3

**ID:** epic-014
**Status:** in_progress
**Erstellt:** 2026-03-10
**Priorität:** high

## Beschreibung
Das gesamte Frontend unter `/src/frontend` wird von Vanilla JavaScript auf TypeScript + Vue.js 3 (Composition API mit `<script setup>`) migriert. Ziel ist langfristige Wartbarkeit, Typsicherheit und eine skalierbare Komponentenarchitektur.

## Akzeptanzkriterien
- [ ] Alle Vanilla-JS-Dateien sind nach TypeScript + Vue.js 3 migriert
- [ ] Webpack-Konfiguration unterstützt Vue.js 3 + TypeScript
- [ ] Alle bestehenden Features funktionieren identisch wie vorher
- [ ] Kein `any`-Type ohne expliziten Kommentar
- [ ] Composition API mit `<script setup>` durchgängig verwendet
- [ ] `cargo build` und `npm run build` laufen fehlerfrei
- [ ] Cargo.toml bleibt unverändert

## Tasks
- [x] task-047: Abhängigkeiten & Build-Konfiguration für Vue.js 3 + TypeScript
- [x] task-048: TypeScript-Typen und Projektstruktur anlegen
- [x] task-049: Vue.js App-Grundgerüst – main.ts, App.vue, Router-loses Setup
- [x] task-050: Migration api.js + state.js → Composables (useApi, useAppState)
- [x] task-051: Migration dom.js → Vue-Komponenten (Layout, Sidebar, Modals)
- [x] task-052: Migration Board-Komponente – jKanban entfernt, **VueDraggablePlus** eingeführt
- [x] task-053: Migration Task-Modal und Task-Detail Komponenten
- [ ] task-054: Migration Column-Modal, Project-Menu, Bulk-Actions
- [ ] task-055: Migration Auth, Admin, Password, Git-Settings, Import, JSON-View
- [ ] task-056: Migration Theme-Toggle, SSE-Service, Utils
- [ ] task-057: Finaler Build-Test, Aufräumen alter JS-Dateien, Verifikation

## Notizen
- Cargo.toml und build.rs bleiben **unverändert**
- Die Idee stammt aus `/vibe/ideas/refactoring-vuejs.md`
- Reihenfolge der Tasks ist wichtig: Basis zuerst, dann komponentenweise Migration
- **jKanban wird durch VueDraggablePlus ersetzt** – besser gepflegt und nativ Vue-kompatibel
- VueDraggablePlus basiert auf SortableJS und unterstützt Vue 3 Composition API
- Die jKanban-Dependency und zugehörige Webpack-Hacks (exports-loader, string-replace-loader) können entfernt werden
