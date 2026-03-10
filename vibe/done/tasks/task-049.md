# Task: Vue.js App-Grundgerüst – main.ts, App.vue, Router-loses Setup

**ID:** task-049
**Epic:** epic-014
**Status:** done
**Erstellt:** 2026-03-10
**Assignee:** developer

## Beschreibung
Das Vue.js 3 App-Grundgerüst aufbauen: main.ts wird zu einem echten Vue-Entry-Point, App.vue wird die Root-Komponente. Die bestehende app.js-Logik (init, checkAuth, Theme) wird in die Vue-Struktur integriert.

## Anforderungen
- [x] `src/frontend/main.ts` umschreiben: Vue-App erstellen und mounten
- [x] `src/frontend/App.vue` als Root-Komponente anlegen
- [x] App.vue enthält die grundlegende Seitenstruktur (Login-View vs. Board-View)
- [x] Bestehende init()-Logik aus app.js in Vue onMounted() migrieren
- [x] Theme-Logik (Dark/Light Mode) als Composable `useTheme.ts` anlegen
- [x] `static/index.html` anpassen: mount-Point `<div id="app"></div>` sicherstellen
- [x] Die App muss nach dem Build funktional identisch laden (Login-Screen oder Board)
- [x] `npm run build` fehlerfrei
- [x] `cargo build` fehlerfrei

## Technische Hinweise
- Bestehende Logik in `app.js` (79 Zeilen) und `theme.js` (23 Zeilen) als Referenz
- Bestehender mount-Point in `static/index.html` prüfen
- Vue mountet auf `#app` – sicherstellen dass dieser Container existiert
- Noch kein Vue-Router nötig – einfache v-if-Logik für Login vs. Board reicht
- Die bestehenden JS-Komponenten können vorerst weiter über DOM-Manipulation arbeiten, bis sie in späteren Tasks migriert werden

## Dev Log
- `main.ts`: Umgeschrieben auf `createApp(App).mount('#app')` – Vue.js 3 Entry-Point
- `App.vue`: Root-Komponente mit `<script setup lang="ts">`, Login-Template (v-if) und Board-Bridging (buildDOM via onMounted)
- `composables/useTheme.ts`: Neues Composable mit initTheme(), toggleTheme(), themeIcon(), reactive currentTheme
- `static/index.html`: `<div id="app"></div>` als Mount-Point hinzugefügt
- Bridging-Strategie: Vue steuert Login vs. Board, bestehende DOM-Manipulation läuft innerhalb des Vue-Lifecycles
- Bundle-Größe: 142 KiB (+ ~62 KiB für Vue.js Runtime)
- `npm run build` + `cargo build` erfolgreich

## Tester Notes
- Build fehlerfrei (npm + cargo)
- Vue.js korrekt eingebunden, App mountet auf #app
- Login-Template als Vue-Template, Board-View über bestehende buildDOM()-Bridge
- useTheme Composable sauber implementiert
- Keine Breaking Changes

## Abnahme
