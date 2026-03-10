# Prompt: Frontend Refactoring – Vanilla JS → TypeScript + Vue.js 3

Bitte lese zuerst `/vibe/readme.md`, um den Workflow dieser App vollständig zu verstehen, bevor du irgendetwas änderst.

---

## Ziel

Das Frontend unter `/src/frontend` wurde bisher vollständig in Vanilla JavaScript geschrieben. Um langfristige Wartbarkeit, Typsicherheit und eine skalierbare Komponentenarchitektur zu gewährleisten, soll das gesamte Frontend auf **TypeScript** und **Vue.js 3** (Composition API mit `<script setup>`) migriert werden.

---

## Rahmenbedingungen

- Das Frontend liegt unter `/src/frontend`
- Es existiert bereits eine `webpack.config.ts` und eine `package.json` – beide sollen für Vue.js 3 + TypeScript erweitert werden
- In der `Cargo.toml` ist der Frontend-Build als Schritt eingebunden – dieser Teil bleibt **unverändert**
- Das Build-Output-Verzeichnis (z. B. `dist/`) sowie alle Pfade, auf die Cargo sich bezieht, bleiben gleich
- Bestehende API-Aufrufe, Event-Strukturen und Backend-Interfaces sollen 1:1 erhalten bleiben und lediglich typisiert werden

---

## Schritt 1: Analyse & Bestandsaufnahme

Bevor du irgendetwas änderst:

1. Lies `/vibe/readme.md` vollständig
2. Analysiere alle Dateien unter `/src/frontend` und erstelle eine kurze Übersicht:
    - Welche JS-Dateien existieren und was tun sie?
    - Welche DOM-Manipulationen, Event-Listener und State-Verwaltung ist vorhanden?
    - Welche Abhängigkeiten werden bereits genutzt?
3. Analysiere `webpack.config.ts` und `package.json` auf den aktuellen Stand
4. Lies `Cargo.toml` und identifiziere den Frontend-Build-Schritt – **ändere diesen nicht**

---

## Schritt 2: Abhängigkeiten & Konfiguration aktualisieren

Erweitere `package.json` und `webpack.config.ts` für Vue.js 3 + TypeScript:

**Neue Dependencies (package.json):**
```json
"vue": "^3.x",
"@vue/compiler-sfc": "^3.x"
```

**Neue DevDependencies (package.json):**
```json
"typescript": "^5.x",
"ts-loader": "^9.x",
"vue-loader": "^17.x",
"vue-tsc": "^2.x",
"@vue/tsconfig": "^0.x"
```

**webpack.config.ts anpassen:**
- Entry Point auf eine neue `main.ts` umstellen
- `vue-loader` und `ts-loader` als Rules hinzufügen
- `VueLoaderPlugin` registrieren
- Resolve-Extensions um `.ts`, `.vue` erweitern
- Alias `@` auf `/src/frontend` setzen

**Neue Konfigurationsdateien anlegen:**
- `tsconfig.json` mit Vue-kompatiblen Einstellungen (`"jsx": "preserve"`, `"moduleResolution": "bundler"`, Vue-Types includen)
- `src/frontend/env.d.ts` für das Deklarieren von `*.vue`-Modulen:
  ```ts
  /// <reference types="vite/client" />
  declare module '*.vue' {
    import type { DefineComponent } from 'vue'
    const component: DefineComponent
    export default component
  }
  ```

---

## Schritt 3: Projektstruktur aufbauen

Lege folgende Struktur unter `/src/frontend` an (soweit noch nicht vorhanden):

```
/src/frontend
├── main.ts              # App-Einstiegspunkt, mountet Vue
├── App.vue              # Root-Komponente
├── components/          # Wiederverwendbare UI-Komponenten
├── composables/         # Wiederverwendbare Logik (useXyz-Pattern)
├── types/               # Gemeinsame TypeScript-Interfaces & Types
├── assets/              # Statische Assets (CSS, Bilder etc.)
└── env.d.ts             # Vue-Modul-Deklaration
```

---

## Schritt 4: Migration der bestehenden Logik

Migriere den bestehenden Vanilla-JS-Code **komponentenweise** nach Vue.js 3:

- **DOM-Manipulation** → reaktive Vue-Templates mit `ref()` und `computed()`
- **Event-Listener** → Vue-Event-Handling (`@click`, `@input`, `v-on`)
- **Globaler State** → `reactive()` / `ref()` in Composables oder einem zentralen Store-Composable
- **Fetch/API-Aufrufe** → in eigene Composables auslagern (z. B. `useApi.ts`), mit vollständiger TypeScript-Typisierung der Request- und Response-Strukturen
- **Klassen-/Style-Manipulation** → `:class` und `:style` Bindings

Nutze konsequent die **Composition API mit `<script setup>`** – keine Options API.

Jede migrierte Datei soll:
- Vollständig typisiert sein (kein `any` ohne expliziten Kommentar)
- Die Original-Funktionalität zu 100% erhalten
- Sinnvoll in Komponenten und Composables aufgeteilt sein

---

## Schritt 5: Build verifizieren

Nach der Migration:

1. `npm install` ausführen
2. `npm run build` (oder das bestehende Build-Skript) ausführen und sicherstellen, dass kein TypeScript-Fehler und kein Webpack-Fehler auftritt
3. Sicherstellen, dass der Cargo-Build-Schritt weiterhin funktioniert und das `dist/`-Verzeichnis (oder das konfigurierte Output-Verzeichnis) korrekt befüllt wird
4. Die App manuell auf Funktionsgleichheit mit dem Vanilla-JS-Stand prüfen

---

## Wichtige Hinweise

- **Cargo.toml bleibt unangetastet** – nur `package.json` und `webpack.config.ts` werden angepasst
- Keine funktionalen Änderungen im Zuge des Refactorings – erst migrieren, dann verbessern
- Sollte dir beim Analysieren ein Bereich unklar sein, frage nach, bevor du mit der Migration beginnst
- Commit-Granularität: Mache nach jedem abgeschlossenen Schritt einen logischen Commit