# Epic-006: Authentifizierung & Nutzerverwaltung

## Status: open

## Beschreibung

Plankton bekommt ein eigenes Login-System ohne externe Auth-Provider.
Alle Endpunkte (außer POST /auth/login) sind geschützt. Zwei Rollen:
admin (Nutzerverwaltung) und user (Board-Zugriff).

## Anforderungen

### Backend (Rust/Axum)

- **Passwort-Hashing**: Argon2id via `argon2` Crate
- **JWT-Sessions**: `jsonwebtoken` Crate, HttpOnly-Cookie, 8h Laufzeit
- **Nutzer-Datenmodell**:
  - id (UUID), username (unique, lowercase), display_name, password_hash
  - role ("admin" | "user"), created_at, updated_at, active (bool)
- **Speicherung**: gleicher Store wie Projekte (CouchDB oder File-Store `data/users/`)

### API-Endpunkte

- POST /auth/login → JWT-Cookie setzen
- POST /auth/logout → Cookie löschen
- GET /auth/me → eigene User-Info
- POST /auth/change-password → eigenes Passwort ändern
- GET /api/admin/users → alle Nutzer (nur admin)
- POST /api/admin/users → Nutzer anlegen (nur admin)
- PUT /api/admin/users/:id → Nutzer editieren (nur admin)
- DELETE /api/admin/users/:id → Nutzer löschen (nur admin)
- PUT /api/admin/users/:id/password → Passwort überschreiben (nur admin)

### Middleware / Auth-Guard

- Alle /api/* Routen: JWT-Cookie prüfen → 401 wenn ungültig
- Admin-Endpunkte: Rolle prüfen → 403 wenn nicht admin
- /mcp/* Routen: Bearer-Token als Alternative zum JWT-Cookie
- Admin kann sich nicht selbst löschen

### Frontend

- Login-Seite (Username + Passwort), passend zum Dark Theme
- App prüft JWT-Cookie bei Laden → Login-Seite wenn ungültig
- Header: eingeloggter Nutzername + Logout-Button
- Admin-Bereich: Nutzerverwaltung (nur für admin sichtbar)
- Passwort-ändern-Dialog für eigenen Account

### Bootstrap / Erster Start

- Beim ersten Start: Standard-Admin anlegen (username: admin, password: admin)
- Beim Login mit Standard-Passwort → Weiterleitung zu Passwort-Ändern

### Log-Integration

- display_name aus JWT wird in Task-Logs als Identität verwendet
- Format: "2025-03-08 14:32 [Frank] Task erstellt"

## Tasks

- [ ] Task-010: Nutzer-Datenmodell & Store-Implementierung (Backend)
- [ ] Task-011: Auth-Endpunkte (login, logout, me, change-password)
- [ ] Task-012: Auth-Middleware & Guard für alle /api/* Routen
- [ ] Task-013: Admin-Endpunkte (CRUD Nutzerverwaltung)
- [ ] Task-014: Bootstrap-Logik (Standard-Admin beim ersten Start)
- [ ] Task-015: Frontend Login-Seite & Auth-Flow
- [ ] Task-016: Frontend Admin-Bereich (Nutzerverwaltung)
- [ ] Task-017: Frontend Passwort-Ändern Dialog
- [ ] Task-018: Log-Integration (display_name in Task-Logs)

## Quelle

/flow/ideas/auth.md
