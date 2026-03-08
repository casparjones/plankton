# Task-024: MCP JSON-RPC Protokoll implementieren

**Epic:** epic-008
**Status:** open
**Rolle:** Developer

## Beschreibung
Bestehenden /mcp/-Endpunkt auf offizielles MCP-Protokoll (JSON-RPC 2.0 über HTTP) umstellen.

## Akzeptanzkriterien
- [ ] POST /mcp → JSON-RPC 2.0 Envelope (method, params, id)
- [ ] initialize/initialized Handshake
- [ ] tools/list → verfügbare Tools als JSON-RPC Response
- [ ] tools/call → Tool ausführen und Ergebnis als JSON-RPC Response
- [ ] Fehler als JSON-RPC Error Codes
- [ ] Bestehende /mcp/tools und /mcp/call bleiben als Legacy erhalten
