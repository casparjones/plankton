# Task: Migration api.js + state.js → Composables (useApi, useAppState)

**ID:** task-050
**Epic:** epic-014
**Status:** done
**Erstellt:** 2026-03-10
**Assignee:** developer

## Beschreibung
Die bestehenden `api.js` (Fetch-Wrapper) und `state.js` (globaler State) nach TypeScript migrieren und als Composables bereitstellen.

## Anforderungen
- [x] `src/frontend/composables/useApi.ts` erstellen: typisierter Fetch-Wrapper mit GET, POST, PUT, DELETE
- [x] `src/frontend/composables/useAppState.ts` erstellen: reaktiver globaler State mit allen Feldern aus state.js
- [x] COLUMN_COLORS Konstante als exportierte Konstante in useAppState.ts
- [x] Alle API-Funktionen vollständig typisiert (Generics für Response-Types)
- [x] Bestehende api.js und state.js bleiben erhalten (legacy-Code referenziert sie)
- [x] `npm run build` fehlerfrei

## Technische Hinweise
- `api.js` (34 Zeilen): Einfacher Fetch-Wrapper mit JSON-Handling
- `state.js` (23 Zeilen): Globales State-Objekt + COLUMN_COLORS
- Die neuen Composables werden in späteren Tasks schrittweise die alten JS-Module ersetzen

## Dev Log
- `composables/useApi.ts`: Typisierter Fetch-Wrapper mit Generics (get<T>, post<T>, put<T>, del)
- `composables/useAppState.ts`: Reaktiver State via Vue reactive(), COLUMN_COLORS als readonly Array
- Beide Module exportieren sowohl Composable-Funktionen als auch direkte Exporte
- Build erfolgreich (142 KiB bundle.js)

## Tester Notes
- API-Client vollständig typisiert, Error-Handling identisch mit Original
- State-Interface aus types/index.ts korrekt verwendet
- COLUMN_COLORS 1:1 übernommen
- Build fehlerfrei

## Abnahme
