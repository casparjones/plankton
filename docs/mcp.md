# MCP-Logik in Plankton

Dieses Dokument beschreibt das Model-Context-Protocol (MCP) in Plankton auf konzeptioneller Ebene – ohne Implementierungsdetails. Es richtet sich an alle, die verstehen möchten, wie KI-Agenten mit dem Board interagieren, wie Sessions und Authentifizierung funktionieren und welche Regeln bei Tool-Aufrufen gelten.

---

## Was ist MCP in Plankton?

MCP ist die Schnittstelle, über die externe KI-Agenten (z.B. claude.ai, Claude Code) das Plankton-Board steuern können. Der Agent ruft benannte Tools auf – z.B. `create_task` oder `move_task` – und erhält strukturierte JSON-Antworten zurück. Aus Sicht des Agents entspricht das dem Aufrufen einer Funktion; aus Sicht von Plankton ist es ein authentifizierter API-Aufruf.

Das Protokoll folgt dem offenen [Model Context Protocol](https://modelcontextprotocol.io/) Standard und nutzt JSON-RPC 2.0 als Nachrichtenformat.

---

## Transportmethoden

Plankton unterstützt zwei Transportmodi über denselben Endpunkt (`/mcp`):

### 1. Stateless (ohne Session)

Ein einzelner HTTP-POST-Request mit einem JSON-RPC-Aufruf, sofortige Antwort. Kein Session-Handshake nötig. Geeignet für einfache Skripte und einmalige Aufrufe.

```
POST /mcp  →  sofortige JSON-Antwort
```

### 2. Streamable HTTP Transport (mit Session)

Persistente Session für mehrere aufeinanderfolgende Aufrufe. Der Client initialisiert zuerst eine Session, erhält eine Session-ID, und sendet alle weiteren Aufrufe mit dieser ID. Zusätzlich kann ein dauerhafter SSE-Kanal für Server-seitige Benachrichtigungen geöffnet werden.

```
POST   /mcp   →  initialize → Session-ID im Response-Header
POST   /mcp   →  tool-Aufrufe mit Mcp-Session-Id Header
GET    /mcp   →  SSE-Stream für Server-Push-Events (optional)
DELETE /mcp   →  Session explizit beenden
```

claude.ai Connectors und MCP-fähige Clients verwenden automatisch den Session-basierten Modus. Der Modus wird durch das Vorhandensein des `initialize`-Calls bestimmt.

---

## Session-Lebenszyklus

```
Client                          Plankton
  │                               │
  │── POST /mcp (initialize) ────▶│  Token prüfen
  │                               │  Session anlegen (UUID)
  │◀─ 200 + Mcp-Session-Id ──────│
  │                               │
  │── POST /mcp (tools/list) ────▶│  Session-ID + Token prüfen
  │◀─ Tool-Liste ─────────────────│
  │                               │
  │── POST /mcp (tools/call) ────▶│  Tool ausführen
  │◀─ Ergebnis ───────────────────│
  │                               │
  │── GET  /mcp (SSE) ───────────▶│  SSE-Stream öffnen (optional)
  │◀─ Server-Events ──────────────│
  │                               │
  │── DELETE /mcp ───────────────▶│  Session entfernen
  │◀─ 200 OK ─────────────────────│
```

**Automatische Session-Erstellung:** Sendet ein Client mit gültigem Token einen Tool-Aufruf *ohne* vorheriges `initialize`, erstellt Plankton automatisch eine neue Session. Clients müssen den Session-Handshake also nicht zwingend explizit durchführen.

**Session-Persistenz:** Sessions werden im Arbeitsspeicher gehalten. Nach einem Serverneustart sind alle Sessions ungültig; Clients müssen sich neu initialisieren.

---

## Authentifizierung

Plankton kennt zwei Wege zur Authentifizierung am MCP-Endpunkt:

| Methode | Wer | Wie |
|---------|-----|-----|
| **JWT-Cookie** | Eingeloggte Browser-Nutzer | Wird automatisch mitgesendet |
| **Agent-Token** | KI-Agenten, CLI, Skripte | `Authorization: Bearer plk_...` Header |

Agent-Tokens werden vom Administrator unter `POST /api/admin/tokens` erstellt. Jeder Token hat einen Namen, eine Rolle und kann optional mit einem Ablaufdatum versehen werden.

**Kein Token → 401 mit `WWW-Authenticate` Header.** Das Format des Headers entspricht RFC 9728 (OAuth Protected Resource Metadata), sodass OAuth-fähige Clients (wie claude.ai) daraus automatisch den OAuth-Login-Flow starten können.

---

## Rollen und Berechtigungen

Jeder Token (und jeder eingeloggte Nutzer) hat eine Rolle. Die Rolle bestimmt, welche Tools im Tool-Listing erscheinen und welche Aufrufe erlaubt sind.

| Rolle | Beschreibung | Tool-Zugang |
|-------|-------------|-------------|
| `admin` | Vollzugriff | Alle Tools |
| `user` | Eingeloggter Browser-Nutzer | Alle Tools |
| `manager` | Projektmanager | Alle Tools außer `delete_task` |
| `developer` | Entwickler | Lesen, eigene Tasks bearbeiten, für Review einreichen |
| `tester` | Tester/QA | Lesen, Review-Queue, genehmigen/ablehnen, kommentieren |

Tools ohne Rollen-Einschränkung (`list_projects`, `get_project`, `get_task`, `summarize_board`, `list_subtasks`) sind für alle authentifizierten Aufrufer sichtbar.

Das Tool-Listing (`tools/list`) gibt immer nur die Tools zurück, die der anfragende Caller tatsächlich verwenden darf.

---

## Tool-Übersicht

### Lesende Tools (alle Rollen)

| Tool | Beschreibung |
|------|-------------|
| `list_projects` | Alle Projekte mit ID, Titel, Slug und Task-Anzahl |
| `get_project` | Projekt mit Spalten und Tasks (kompakt: ohne Kommentare/Logs) |
| `get_task` | Vollständige Task-Details inkl. Kommentare und Logs |
| `summarize_board` | Board-Übersicht: Spalten mit Task-Anzahl je Spalte |
| `list_subtasks` | Subtasks eines Epics mit Fertigstellungsstatus |

### Projekt-Management (manager, admin)

| Tool | Beschreibung |
|------|-------------|
| `create_project` | Neues Projekt mit Standard-Spalten anlegen |
| `update_project` | Titel, Typ, Ablauf-Regeln, Pinning aktualisieren |
| `list_epics` | Spalten als Epics mit Task-Anzahl (geordnet) |

### Task-Lifecycle (developer, manager, admin)

| Tool | Beschreibung |
|------|-------------|
| `create_task` | Neuen Task anlegen (landet in erster Spalte) |
| `create_task_from_template` | Task aus vordefinierten oder eigenen Templates erstellen |
| `update_task` | Titel, Beschreibung, Labels, Worker, Punkte ändern |
| `move_task` | Task in eine andere Spalte verschieben |
| `move_task_to_project` | Task in ein anderes Projekt verschieben |
| `reorder_tasks` | Reihenfolge der Tasks innerhalb einer Spalte ändern |
| `assign_task` | Bearbeiter zuweisen |
| `delete_task` | Task löschen (nur manager, admin) |

### Eigene Tasks (developer)

| Tool | Beschreibung |
|------|-------------|
| `get_assigned_tasks` | Tasks abrufen, die dem Aufrufer zugewiesen sind |
| `submit_for_review` | Task aus "In Progress" in die Testing-Spalte einreichen |

### Review-Flow (tester, manager, admin)

| Tool | Beschreibung |
|------|-------------|
| `get_review_queue` | Alle Tasks mit Label "review" (auf Tester wartend) |
| `approve_task` | Task abnehmen → wird nach "Done" verschoben |
| `reject_task` | Task zurückweisen → zurück in die vorherige Spalte + Kommentar |

### Kommunikation (developer, tester, manager, admin)

| Tool | Beschreibung |
|------|-------------|
| `add_comment` | Kommentar zu einem Task hinzufügen (primäres Kommunikationstool) |
| `add_log` | **Veraltet** – Alias für `add_comment`, aus Rückwärtskompatibilität erhalten |

### Relationen (developer, manager, admin)

| Tool | Beschreibung |
|------|-------------|
| `add_relation` | Abhängigkeit (`blocks`) oder Eltern-Kind-Beziehung (`subtask`) erstellen |
| `remove_relation` | Relation entfernen |

---

## Workflow-Muster

### Standard Kanban-Flow

```
Todo → In Progress → Testing → Done
```

1. **Manager** erstellt Tasks (`create_task`) und weist sie zu (`assign_task`).
2. **Developer** holt sich seine Tasks (`get_assigned_tasks`), verschiebt sie nach "In Progress" (`move_task`) und dokumentiert Fortschritt (`add_comment`).
3. **Developer** reicht zur Review ein (`submit_for_review`) → Task wandert nach "Testing".
4. **Tester** prüft die Review-Queue (`get_review_queue`) und genehmigt (`approve_task` → Done) oder lehnt ab (`reject_task` → zurück zu In Progress + Kommentar).

### Epic / Subtask-Struktur

Ein Epic ist ein Task vom Typ `epic`. Subtasks werden per `add_relation` mit `relation: "subtask"` verknüpft. Die Verknüpfung ist **bidirektional**: Das Epic kennt seine `subtask_ids`, der Subtask kennt seine `parent_id`.

```
Epic A (task_type: "epic")
  ├── Subtask 1 (parent_id: epic_a_id)
  └── Subtask 2 (parent_id: epic_a_id)
```

### Abhängigkeiten (Blocking)

Mit `add_relation` und `relation: "blocks"` wird Task A als Blocker für Task B markiert. Solange Task A nicht in der "Done"-Spalte ist, kann Task B **nicht** nach "In Progress" verschoben werden – Plankton erzwingt dies beim `move_task`-Aufruf und gibt einen Fehler mit den Namen der offenen Blocker zurück.

---

## Optimistic Locking

Mehrere Agents können gleichzeitig dasselbe Projekt bearbeiten. Um Race-Conditions zu verhindern, verwendet Plankton einen zweilagigen Mechanismus:

1. **Write-Lock pro Projekt**: Alle schreibenden Operationen auf einem Projekt serialisiert Plankton intern. Zwei gleichzeitige Schreibaufrufe auf dasselbe Projekt warten aufeinander.

2. **Optionaler `_rev`-Check**: Der Caller kann beim Abruf eines Projekts oder Tasks den Revisionswert (`_rev`) merken und bei einem späteren Schreibaufruf mitschicken. Hat sich das Projekt zwischenzeitlich geändert, antwortet Plankton mit `409 Conflict` und dem aktuellen Revisionswert. Der Agent kann dann das Objekt neu laden und die Änderung erneut versuchen.

Tools, die `_rev` unterstützen: `update_task`, `move_task`, `assign_task`, `delete_task`.

---

## Task-Templates

Mit `create_task_from_template` können Tasks aus vorgefertigten Vorlagen erstellt werden. Plankton sucht Templates in dieser Reihenfolge:

1. **Lokale Datei**: `.plankton/templates/<name>.json` im Arbeitsverzeichnis des Servers
2. **Eingebaute Defaults**: `bug`, `feature`, `security`, `epic`, `chore`

Jede Vorlage kann die Platzhalter `{{title}}` und `{{date}}` im Titel und in der Beschreibung verwenden. Der `{{date}}`-Platzhalter wird mit dem aktuellen Datum im Format `YYYY-MM-DD` ersetzt.

Eigene Templates folgen dem gleichen JSON-Format wie die Eingebauten:

```json
{
  "title": "CHORE: {{title}}",
  "task_type": "task",
  "labels": ["chore"],
  "description": "## Aufgabe\n\n## Erstellt am\n\n{{date}}"
}
```

---

## Echtzeit-Events (SSE)

Nach dem Öffnen eines SSE-Streams (`GET /mcp` mit Session-ID) sendet Plankton folgende Events an den Client:

| Event-Typ | Auslöser |
|-----------|---------|
| `task_created` | Neuer Task wurde angelegt |
| `task_updated` | Task wurde geändert (Felder, Kommentar, Log) |
| `task_moved` | Task wurde in eine andere Spalte verschoben |
| `task_deleted` | Task wurde gelöscht |
| `project_updated` | Projekt-Metadaten wurden geändert |
| `heartbeat` | Regelmäßiges Ping-Event bei zu langsamer Leserate |

SSE-Events enthalten die vollständigen Task-Daten im JSON-Format. Der Stream bleibt offen, bis die Session per `DELETE /mcp` beendet wird oder die Verbindung getrennt wird.

---

## Fehlerbehandlung

Plankton antwortet mit standardkonformen JSON-RPC-2.0-Fehlern:

| Code | Bedeutung | Wann |
|------|-----------|------|
| `-32700` | Parse error | HTTP-Body ist kein gültiges JSON |
| `-32600` | Invalid request | JSON ist kein gültiges JSON-RPC-Objekt |
| `-32601` | Method not found | Unbekannte RPC-Methode (z.B. Tippfehler) |
| `-32000` | Tool-Fehler | Fehler bei der Tool-Ausführung (z.B. Task not found, Bad Request) |

HTTP-Status-Codes zusätzlich:

| Status | Bedeutung |
|--------|-----------|
| `200` | Erfolgreiche Antwort |
| `202 Accepted` | Notification ohne Antwort (kein `id`-Feld im Request) |
| `400` | Fehlender Session-Header oder ungültige Parameter |
| `401` | Kein Token oder ungültiger Token |
| `404` | Session nicht gefunden |
| `409 Conflict` | Optimistic-Locking-Fehler (`_rev` veraltet) |

---

## Integration

### claude.ai Connector

Plankton unterstützt OAuth 2.0 Discovery (RFC 8414, RFC 9728). claude.ai erkennt die OAuth-Endpoints automatisch über `/.well-known/oauth-authorization-server`.

**Setup in claude.ai**: Settings → Connectors → Add custom connector → URL: `https://<plankton-host>/mcp`

### Claude Code (MCP-Config)

```json
{
  "mcpServers": {
    "plankton": {
      "url": "https://<plankton-host>/mcp",
      "transport": "http",
      "headers": {
        "Authorization": "Bearer plk_<token>"
      }
    }
  }
}
```

### Claude Code Skill

```bash
plankton skill install https://<plankton-host> --global
```

Der Skill wird unter `~/.claude/skills/plankton.md` installiert und enthält die vollständige Tool-Referenz sowie Anweisungen für den Agenten.

---

## Protokoll-Kompatibilität

Plankton unterstützt zwei MCP-Protokollversionen:

- `2024-11-05` – Ältere Version, breite Client-Kompatibilität
- `2025-03-26` – Streamable HTTP Transport, von claude.ai bevorzugt

Der Server übernimmt immer die vom Client gesendete `protocolVersion` in der `initialize`-Antwort. Beide Versionen werden vollständig unterstützt.

CORS ist permissiv konfiguriert – alle Origins sind erlaubt. Das ermöglicht Browser-basierte MCP-Clients und claude.ai-Integrationen ohne Einschränkungen.
