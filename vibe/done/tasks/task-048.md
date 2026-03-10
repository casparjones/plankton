# Task: TypeScript-Typen und Projektstruktur anlegen

**ID:** task-048
**Epic:** epic-014
**Status:** done
**Erstellt:** 2026-03-10
**Assignee:** developer

## Beschreibung
Zentrale TypeScript-Interfaces und -Types für alle Datenstrukturen anlegen, die im Frontend verwendet werden. Außerdem die Vue.js-Verzeichnisstruktur vorbereiten.

## Anforderungen
- [x] `src/frontend/types/` Verzeichnis anlegen
- [x] `src/frontend/types/index.ts` mit allen Interfaces: Project, Column, Task, User, GitConfig, AuthUser, etc. (basierend auf den Rust-Models in `src/models/project.rs` und `src/models/auth.rs`)
- [x] `src/frontend/composables/` Verzeichnis anlegen
- [x] `src/frontend/components/` Verzeichnis existiert bereits – prüfen
- [x] `src/frontend/assets/` Verzeichnis anlegen (für zukünftige statische Assets)
- [x] Alle Types müssen die bestehende API-Struktur 1:1 abbilden
- [x] `npm run build` läuft weiterhin fehlerfrei

## Technische Hinweise
- Rust-Datenmodelle in `src/models/project.rs` (146 Zeilen) und `src/models/auth.rs` als Referenz nutzen
- Bestehende Felder in `state.js` beachten
- Die API-Responses (z.B. GET /api/projects/:id) definieren die Struktur
- Alle string-Felder die im Rust-Backend `Option<String>` sind, werden im TS als `string | null` abgebildet

## Dev Log
- `src/frontend/types/index.ts`: 18 Interfaces erstellt – ProjectDoc, GitConfig, Column, User, Task, AuthUser, Claims, LoginRequest, ChangePasswordRequest, CreateAuthUserRequest, UpdateAuthUserRequest, ResetPasswordRequest, AgentToken, CreateTokenRequest, UpdateTokenRequest, AppState, ApiError, MoveTaskRequest
- Alle Felder 1:1 aus den Rust-Models übernommen, Option<String> → string | null
- Frontend-spezifische Typen (AppState, ApiError, MoveTaskRequest) zusätzlich angelegt
- Verzeichnisse erstellt: types/, composables/, assets/
- `npm run build` erfolgreich

## Tester Notes
- Types geprüft: 1:1 Abbildung der Rust-Models bestätigt
- Verzeichnisstruktur korrekt angelegt
- Build fehlerfrei

## Abnahme
