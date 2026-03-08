# Task-012: Auth-Middleware & Guard für alle /api/* Routen

**Epic:** epic-006
**Status:** open
**Rolle:** Developer

## Beschreibung
Axum-Middleware die JWT-Cookie bei allen /api/* Requests prüft. Nicht eingeloggte Requests → 401. /mcp/* akzeptiert auch Bearer-Token. Statische Dateien und /auth/* sind öffentlich.

## Akzeptanzkriterien
- [ ] auth_guard Middleware: JWT aus Cookie oder Authorization Bearer Header extrahieren
- [ ] JWT validieren und Claims in Request-Extensions speichern
- [ ] Ungültiger/fehlender Token → 401 JSON Response
- [ ] /auth/login und /auth/logout sind öffentlich (kein Guard)
- [ ] Statische Dateien (/, /index.html, etc.) sind öffentlich
- [ ] Handler können auf Claims zugreifen (user_id, role)
