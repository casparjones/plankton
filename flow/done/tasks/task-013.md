# Task-013: Admin-Endpunkte (CRUD Nutzerverwaltung)

**Epic:** epic-006
**Status:** open
**Rolle:** Developer

## Beschreibung
Admin-only Endpunkte für Nutzerverwaltung. Nur Nutzer mit role=admin dürfen diese verwenden.

## Akzeptanzkriterien
- [ ] GET /api/admin/users → alle Nutzer auflisten (ohne password_hash)
- [ ] POST /api/admin/users → neuen Nutzer anlegen (username, display_name, password, role)
- [ ] PUT /api/admin/users/:id → Nutzer editieren (display_name, role, active)
- [ ] DELETE /api/admin/users/:id → Nutzer löschen
- [ ] PUT /api/admin/users/:id/password → Passwort überschreiben
- [ ] Admin kann sich nicht selbst löschen → 400 Bad Request
- [ ] Nicht-Admin → 403 Forbidden
