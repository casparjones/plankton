# MCP-Server (JSON-RPC 2.0)

Plankton implementiert das [Model Context Protocol](https://modelcontextprotocol.io/) und stellt 30+ Tools für KI-Agenten bereit.

## Transport-Methoden

### 1. Stateless JSON-RPC (POST /mcp)

Einfachster Ansatz: ein Request, eine Response.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "list_projects",
  "params": {},
  "id": "req-1"
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": [...],
  "id": "req-1"
}
```

**Fehler-Response:**
```json
{
  "jsonrpc": "2.0",
  "error": {"code": -32600, "message": "..."},
  "id": "req-1"
}
```

### 2. Streamable HTTP (GET /mcp + POST /mcp)

Für persistente Sessions mit Server-Push (z.B. claude.ai Connector).

1. `GET /mcp` mit Bearer-Token → Server erstellt Session, antwortet mit `text/event-stream`
2. Client sendet Requests via `POST /mcp` mit Header `X-Mcp-SessionId: <id>`
3. Server streamt Results und Events zurück auf den SSE-Kanal
4. `DELETE /mcp` beendet die Session

---

## Tool-Referenz

### Projekt-Tools

| Tool | Params | Beschreibung |
|------|--------|-------------|
| `list_projects` | — | Alle Projekte (id, title, slug, task_count) |
| `get_project` | `project_id` | Projekt mit Spalten und Tasks (kompakt) |
| `create_project` | `title` | Neues Projekt mit Standard-Spalten |
| `update_project` | `project_id`, `title?`, `description?` | Metadaten aktualisieren |
| `list_epics` | `project_id` | Spalten mit Task-Anzahl |
| `summarize_board` | `project_id` | Board-Übersicht (Spalten + Counts) |

### Task-Tools

| Tool | Params | Beschreibung |
|------|--------|-------------|
| `get_task` | `project_id`, `task_id` | Vollständige Task-Details inkl. Logs und Comments |
| `create_task` | `project_id`, `title`, `description?`, `task_type?` | Neue Task (landet in erster Spalte) |
| `update_task` | `project_id`, `task_id`, `title?`, `description?`, `labels?`, `points?`, `worker?` | Partielles Update |
| `delete_task` | `project_id`, `task_id` | Task löschen |
| `move_task` | `project_id`, `task_id`, `column_id` | In andere Spalte verschieben |
| `reorder_tasks` | `project_id`, `column_id`, `task_ids[]` | Reihenfolge in Spalte ändern |
| `assign_task` | `project_id`, `task_id`, `user_id` | Bearbeiter zuweisen |
| `get_assigned_tasks` | `project_id`, `user_id?` | Zugewiesene Tasks abrufen |

### Review/Workflow-Tools

| Tool | Params | Beschreibung |
|------|--------|-------------|
| `submit_for_review` | `project_id`, `task_id` | Task in Testing-Spalte verschieben |
| `approve_task` | `project_id`, `task_id` | Task genehmigen → Done |
| `reject_task` | `project_id`, `task_id`, `reason` | Task ablehnen → In Progress + Kommentar |
| `get_review_queue` | `project_id` | Tasks die auf Review warten |

### Kommunikations-Tools

| Tool | Params | Beschreibung |
|------|--------|-------------|
| `add_comment` | `project_id`, `task_id`, `text`, `author?` | Kommentar zu Task hinzufügen |
| `add_log` | `project_id`, `task_id`, `message` | Log-Eintrag (delegiert intern zu add_comment) |

### Relation-Tools

| Tool | Params | Beschreibung |
|------|--------|-------------|
| `add_relation` | `project_id`, `task_id`, `target_id`, `type` | Beziehung erstellen: `blocks` oder `subtask` |
| `remove_relation` | `project_id`, `task_id`, `target_id`, `type` | Beziehung entfernen |
| `list_subtasks` | `project_id`, `epic_id` | Subtasks eines Epics auflisten |

---

## Rollen-Basierte Zugriffskontrolle

| Rolle | Erlaubte Tool-Gruppen |
|-------|----------------------|
| `admin`, `user` | Alle Tools |
| `manager` | Alles außer `delete_task` |
| `developer` | `get_*`, `list_*`, `update_task`, `submit_for_review`, `add_*`, `assign_task`, `add_relation`, `remove_relation` |
| `tester` | `get_*`, `list_*`, `get_review_queue`, `approve_task`, `reject_task`, `add_comment` |
| Kein Auth | `list_*`, `get_*`, `summarize_board` |

---

## Integration in claude.ai

Plankton unterstützt OAuth 2.0 Discovery (RFC 8414 + RFC 9728):

1. In claude.ai: **Settings → Connectors → Add custom connector**
2. URL: `https://plankton.tiny-dev.de/mcp`
3. OAuth-Popup öffnet sich → Einloggen → Zugriff erlauben

claude.ai erkennt automatisch die OAuth-Endpoints via `/.well-known/oauth-authorization-server`.

## Integration in Claude Code

```bash
plankton skill install https://plankton.tiny-dev.de --global
```

Installiert den Plankton-Skill in `~/.claude/skills/plankton.md`. Claude Code verwendet den Skill automatisch wenn der Nutzer Plankton-Aufgaben beschreibt.

## Integration via MCP-Config (JSON)

```json
{
  "mcpServers": {
    "plankton": {
      "url": "https://plankton.tiny-dev.de/mcp",
      "transport": "http",
      "headers": {
        "Authorization": "Bearer plk_<your-agent-token>"
      }
    }
  }
}
```
