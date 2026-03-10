# Task: Abhängigkeiten & Build-Konfiguration für Vue.js 3 + TypeScript

**ID:** task-047
**Epic:** epic-014
**Status:** done
**Erstellt:** 2026-03-10
**Assignee:** developer

## Beschreibung
Die bestehende `package.json` und `webpack.config.js` müssen für Vue.js 3 + TypeScript erweitert werden. Neue Konfigurationsdateien (`tsconfig.json`, `env.d.ts`) müssen angelegt werden.

## Anforderungen
- [x] `package.json` erweitern: vue, @vue/compiler-sfc als Dependencies
- [x] `package.json` erweitern: typescript, ts-loader, vue-loader@17, vue-tsc, @vue/tsconfig als DevDependencies
- [x] `webpack.config.js` anpassen: Entry auf `main.ts` umstellen, vue-loader + ts-loader Rules, VueLoaderPlugin, Resolve-Extensions (.ts, .vue), Alias @ auf src/frontend
- [x] `tsconfig.json` anlegen mit Vue-kompatiblen Einstellungen
- [x] `src/frontend/env.d.ts` anlegen für *.vue Modul-Deklarationen
- [x] `npm install` läuft fehlerfrei
- [x] `npm run build` läuft fehlerfrei (auch wenn noch kein Vue-Code existiert, muss der Build mit leerem main.ts durchlaufen)
- [x] Cargo.toml bleibt **unverändert**
- [x] build.rs bleibt **unverändert**

## Technische Hinweise
- Aktuelle webpack.config.js liegt unter `/work/work-data/projects/plankton/webpack.config.js`
- Aktuelle package.json liegt unter `/work/work-data/projects/plankton/package.json`
- Entry-Point ist aktuell `./src/frontend/main.js` → wird zu `./src/frontend/main.ts`
- Der bestehende main.js muss vorerst als main.ts kopiert/umbenannt werden (minimale Änderung)
- Output-Pfad für bundle.js/bundle.css muss gleich bleiben (`static/`)

## Dev Log
- `package.json`: vue 3.5.13 und @vue/compiler-sfc als Dependencies hinzugefügt; typescript, ts-loader, vue-loader@17, vue-tsc, @vue/tsconfig als DevDependencies
- `webpack.config.js`: Entry auf `main.ts` umgestellt, vue-loader + ts-loader Rules hinzugefügt, VueLoaderPlugin registriert, Resolve-Extensions (.ts, .js, .vue, .json), Alias @ auf src/frontend
- `tsconfig.json`: Neu angelegt mit target ES2020, moduleResolution bundler, strict mode, Vue-kompatibel
- `src/frontend/env.d.ts`: Neu angelegt mit *.vue Modul-Deklaration
- `src/frontend/main.ts`: Neu angelegt als TypeScript-Entry-Point (importiert bestehenden app.js mit @ts-ignore)
- `main.js` bleibt vorerst bestehen für Rückwärtskompatibilität
- `npm install` + `npm run build` erfolgreich (bundle.js 79.7 KiB, bundle.css 32.2 KiB)
- `cargo build` erfolgreich (2 vorbestehende Warnings, keine neuen)
- Cargo.toml und build.rs: **unverändert**

## Tester Notes
- Alle Anforderungen geprüft und erfüllt
- `npm run build` kompiliert fehlerfrei (bundle.js 79.7 KiB, bundle.css 32.2 KiB)
- `cargo build` kompiliert fehlerfrei (nur 2 vorbestehende Warnings)
- Cargo.toml und build.rs unverändert bestätigt
- Code-Review: webpack.config.js korrekt konfiguriert, tsconfig.json Vue-kompatibel, env.d.ts sauber

## Abnahme
