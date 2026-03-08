# Idee: Eigenes Login-System (kein externer Auth-Provider)

## Kernidee

Plankton bekommt ein selbst gebautes Authentifizierungssystem.
Kein OAuth, kein Keycloak, kein externer Service – alles läuft im eigenen Stack.
Die App ist ohne Login komplett unzugänglich (kein einziger Endpunkt öffentlich
außer POST /auth/login).

## Rollen

**admin:**
- Kann alles was "user" kann
- Kann neue Nutzer anlegen
- Kann Nutzer löschen
- Kann Nutzer editieren (Name, Rolle)
- Kann das Passwort von jedem Nutzer überschreiben (ohne das alte zu kennen)
- Kann sich nicht selbst löschen

**user:**
- Kann sich einloggen
- Kann alle Projekte und Boards sehen und bearbeiten
- Kann Tasks anlegen, editieren, verschieben, löschen
- Kann sein eigenes Passwort ändern (altes Passwort erforderlich)
- Kann NICHT andere Nutzer verwalten

## Technische Anforderungen

**Passwort-Sicherheit:**
- Passwörter werden mit Argon2id gehasht gespeichert (nie Plaintext, nie MD5/SHA1)
- Argon2id ist der aktuelle Goldstandard für Passwort-Hashing
- Rust-Crate: `argon2`

**Session / Token:**
- Nach erfolgreichem Login bekommt der Nutzer einen signierten JWT (JSON Web Token)
- JWT enthält: user_id, name, role, exp (Ablaufzeit)
- JWT wird als HttpOnly-Cookie gesetzt (sicherer als localStorage)
- Token-Laufzeit: 8 Stunden, danach erneuter Login erforderlich
- Rust-Crate: `jsonwebtoken`

**Nutzer-Datenmodell:**
```
User {
    id:           String,     // UUID
    username:     String,     // eindeutig, lowercase
    display_name: String,     // wird in Logs angezeigt: "Frank"
    password_hash: String,    // Argon2id Hash
    role:         String,     // "admin" | "user"
    created_at:   String,
    updated_at:   String,
    active:       bool,       // deaktivierte Nutzer können sich nicht einloggen
}
```

**API-Endpunkte:**
- POST /auth/login          → username + password → JWT-Cookie setzen
- POST /auth/logout         → Cookie löschen
- GET  /auth/me             → eigene User-Info zurückgeben
- POST /auth/change-password → eigenes Passwort ändern (user + admin)

- GET    /api/admin/users         → alle Nutzer auflisten (nur admin)
- POST   /api/admin/users         → neuen Nutzer anlegen (nur admin)
- PUT    /api/admin/users/:id     → Nutzer editieren (nur admin)
- DELETE /api/admin/users/:id     → Nutzer löschen (nur admin)
- PUT    /api/admin/users/:id/password → Passwort überschreiben (nur admin)

**Middleware:**
- Alle bestehenden /api/* Routen bekommen einen Auth-Guard
- Alle bestehenden /mcp/* Routen akzeptieren weiterhin den Agenten-Token
  (Bearer Token im Header) als Alternative zum JWT-Cookie
- Nicht eingeloggte Requests auf /api/* → 401 Unauthorized
- Requests von "user"-Rolle auf Admin-Endpunkte → 403 Forbidden

**Frontend:**
- Login-Seite: Username + Passwort Formular, schlicht, zum App-Theme passend
- Beim Laden der App: prüfen ob JWT-Cookie gültig, sonst → Login-Seite
- Header zeigt den eingeloggten Nutzernamen + Logout-Button
- Admin-Bereich in den Settings: Nutzerverwaltung (nur sichtbar für admin)
- Passwort-Ändern Dialog für den eigenen Account

**Erster Start (Bootstrap):**
- Beim ersten Start prüft Plankton ob ein Admin-Nutzer existiert
- Wenn nicht: Standard-Admin wird angelegt
    - username: admin
    - password: admin (MUSS beim ersten Login geändert werden)
- Beim ersten Login mit Standard-Passwort → Weiterleitung zu "Passwort ändern"

**Integration mit bestehendem Identity-System:**
- Der display_name aus dem JWT wird als Identität in Task-Logs verwendet
- Format identisch zu Agenten-Logs:
  "2025-03-08 14:32 [Frank] Task erstellt"
  "2025-03-08 14:45 [Claude Developer] Status → in_progress"
- Mensch und Agent sind im Log nicht zu unterscheiden – beide sind einfach ein Name

**Speicherung:**
- Nutzer werden im gleichen Store wie Projekte gespeichert (CouchDB oder File-Store)
- Im File-Store: data/users/<id>.json
- In CouchDB: eigene users-Collection oder Dokumente mit type: "user"