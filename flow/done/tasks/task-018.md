# Task-018: Log-Integration (display_name in Task-Logs)

**Epic:** epic-006
**Status:** open
**Rolle:** Developer

## Beschreibung
display_name aus dem JWT wird als Identität in Task-Logs und creator/worker-Feldern verwendet.

## Akzeptanzkriterien
- [ ] create_task: creator = display_name aus JWT Claims (statt "anonymous")
- [ ] move_task: Log-Format "[Display Name] moved from X to Y"
- [ ] Frontend: worker-Feld zeigt eingeloggten User als Vorschlag
- [ ] MCP-Calls: Bearer-Token → display_name auflösen
