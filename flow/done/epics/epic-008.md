# Epic-008: MCP-Server für LLM-Agenten

## Status: open

## Beschreibung

Plankton wird zu einem vollwertigen MCP-Server (Model Context Protocol),
damit LLM-Agenten (Claude Code, Claude Desktop, Cursor, etc.) direkt
mit dem Board arbeiten können. Der bisherige /flow/-Dateisystem-Workflow
wird langfristig durch native Plankton-Integration ersetzt.

## Anforderungen

### Token-System

- Tokens im Frontend generierbar (Name + Rolle)
- Jeder Token repräsentiert einen Agenten (z.B. "Claude Manager", "Claude Developer")
- Authorization: Bearer <token> bei jedem API-Call
- Plankton resolved automatisch creator/worker aus dem Token
- Log-Format: "2025-03-08 14:32 [Claude Manager] Task erstellt"
- Tokens können deaktiviert/widerrufen werden

### MCP-Protokoll

- Echtes MCP (JSON-RPC über HTTP/SSE) in main.rs implementieren
- Bestehenden /mcp/-Endpunkt auf offizielles MCP-Protokoll umstellen
- Rollenbasierte Tool-Sichtbarkeit:
  - **Manager**: list_epics, create_epic, create_task, assign_task, close_epic
  - **Developer**: get_assigned_tasks, update_task, add_log, submit_for_review
  - **Tester**: get_review_queue, add_comment, approve_task, reject_task

### Dokumentations-Seite

- Statische HTML-Seite unter /docs
- Maschinenlesbar optimiert (klare Struktur, keine Grafiken)
- Inhalt: API-Referenz, Tool-Liste, Workflow-Beschreibung, Token-Setup
- Kann als System-Prompt-Quelle für Agenten dienen

### Ziel

Agenten verbinden sich direkt per MCP zu Plankton, holen ihren nächsten
Task, arbeiten ihn ab, loggen alles und übergeben an den nächsten Agenten.
Alles sichtbar im Board, nachvollziehbar über Logs.

## Tasks

- [ ] Task-022: Token-Datenmodell & CRUD-Endpunkte (Backend)
- [ ] Task-023: Token-Generierung & Verwaltung (Frontend)
- [ ] Task-024: MCP JSON-RPC Protokoll implementieren (Backend)
- [ ] Task-025: Rollenbasierte Tool-Registrierung & Sichtbarkeit
- [ ] Task-026: Agenten-Workflow Tools (Manager/Developer/Tester)
- [ ] Task-027: /docs Seite (maschinenlesbare API-Dokumentation)

## Quelle

/flow/ideas/mcp-functionality.md
