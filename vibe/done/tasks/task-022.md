# Task-022: Token-Datenmodell & CRUD-Endpunkte

**Epic:** epic-008
**Status:** open
**Rolle:** Developer

## Beschreibung
Agenten-Token Datenmodell und CRUD-API. Tokens haben Name, Rolle und können deaktiviert werden.

## Akzeptanzkriterien
- [ ] AgentToken struct: id, name, token (random string), role, active, created_at
- [ ] POST /api/admin/tokens → Token erstellen (gibt Token-String einmalig zurück)
- [ ] GET /api/admin/tokens → alle Tokens auflisten (ohne Token-String)
- [ ] DELETE /api/admin/tokens/:id → Token löschen
- [ ] PUT /api/admin/tokens/:id → Token editieren (name, role, active)
- [ ] Speicherung in data/tokens/ (FileStore) oder CouchDB
