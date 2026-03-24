# 🪼 Plankton

Kanban-Board für KI-Agenten mit MCP-Integration, OAuth 2.0 und Multi-Agent-Workflow.

Rust-Backend (Axum) + TypeScript/Vue-Frontend. Speichert in CouchDB oder lokal als JSON-Dateien.

## Features

- **MCP Server** – Model Context Protocol für claude.ai, Claude Code und andere MCP-Clients
- **OAuth 2.0** – Authorization Code Flow mit PKCE, Dynamic Client Registration (RFC 7591)
- **Multi-Agent Workflow** – Architect, Developer, Tester mit rollenbasiertem Tool-Zugriff
- **CLI** – Multi-Server-fähig mit Device Auth Flow (`plankton remote add`)
- **Claude Code Skill** – Automatische Installation via `plankton skill install`
- **REST-API + SSE** – Echtzeit-Updates, Import/Export, Projekt-Verwaltung

## Quickstart

```bash
# Server starten
cargo run

# Mit CouchDB:
COUCHDB_URI=http://admin:password@localhost:5984 cargo run

# Anderen Port:
PORT=8080 cargo run
```

Browser: **http://localhost:3000**
Default-Login: `admin` / `admin` (Passwortänderung beim ersten Login)

## CLI installieren

```bash
# CLI installieren
curl -fsSL https://plankton.tiny-dev.de/install | bash

# Server hinzufügen + einloggen
plankton remote add origin https://plankton.tiny-dev.de

# Claude Code Skill installieren
plankton skill install https://plankton.tiny-dev.de --global
```

## claude.ai Connector

Plankton lässt sich als MCP-Connector in claude.ai einbinden:

1. In claude.ai: **Settings → Connectors → Add custom connector**
2. URL eingeben: `https://plankton.tiny-dev.de/mcp`
3. Login im OAuth-Popup → Zugriff erlauben → fertig

Claude.ai erkennt automatisch die OAuth-Endpoints via Discovery (RFC 8414 + RFC 9728).

## MCP Tools

| Tool | Beschreibung |
|------|-------------|
| `list_projects` | Alle Projekte (id, title, slug, task_count) |
| `get_project` | Projekt mit Spalten und Tasks (kompakt) |
| `get_task` | Vollständige Task-Details mit Comments/Logs |
| `create_task` | Task erstellen (landet in Todo) |
| `update_task` | Task bearbeiten (Titel, Beschreibung, Labels, ...) |
| `move_task` | Task in andere Spalte verschieben |
| `delete_task` | Task löschen |
| `assign_task` | Worker zuweisen |
| `add_comment` | Kommentar hinzufügen |
| `add_log` | Log-Eintrag schreiben |
| `submit_for_review` | Task zur Review einreichen |
| `approve_task` | Task abnehmen → Done |
| `reject_task` | Task zurückweisen → In Progress |
| `add_relation` | Relation erstellen (blocks, subtask) |
| `list_subtasks` | Subtasks eines Epics auflisten |
| `summarize_board` | Board-Übersicht (Spalten + Task-Anzahl) |

## OAuth 2.0 Endpoints

| Endpoint | Beschreibung |
|----------|-------------|
| `GET /.well-known/oauth-authorization-server` | Server Metadata (RFC 8414) |
| `GET /.well-known/oauth-protected-resource` | Protected Resource Metadata (RFC 9728) |
| `POST /oauth/register` | Dynamic Client Registration (RFC 7591) |
| `GET /oauth/authorize` | Authorization Endpoint (mit Consent-Screen) |
| `POST /oauth/token` | Token Endpoint (PKCE + Refresh Token Rotation) |

## Projektstruktur

```
plankton/
├── src/
│   ├── main.rs                    # Router, Server-Bootstrap
│   ├── controllers/
│   │   ├── mcp_controller.rs      # MCP JSON-RPC + SSE
│   │   ├── oauth_controller.rs    # OAuth 2.0 Flow
│   │   ├── auth_controller.rs     # Login, JWT, Sessions
│   │   ├── admin_controller.rs    # User/Token-Verwaltung
│   │   ├── cli_controller.rs      # CLI + Device Auth + Installer
│   │   └── ...                    # REST-API Controller
│   ├── models/                    # Datenmodelle (Auth, Project, Task)
│   ├── services/                  # Auth-Service, Projekt-Service
│   ├── store/                     # CouchDB + File-Store Backend
│   └── frontend/                  # TypeScript/Vue SPA
├── static/                        # Bundle, CSS, Icons
├── data/
│   ├── projects/                  # Projekt-JSON-Dateien
│   ├── users/                     # User-Dateien
│   ├── tokens/                    # Agent-Token-Dateien
│   └── oauth/                     # OAuth Codes, Clients, Refresh Tokens
└── test-mcp.fish                  # OAuth Flow Test-Script
```

## Entwicklung

```bash
# Frontend im Watch-Modus
npm run dev

# Rust-Server
cargo run

# Tests
fish test-mcp.fish                 # OAuth Flow gegen Production
fish test-mcp-local.fish           # OAuth Flow gegen localhost
```

## Deployment

Plankton läuft als einzelner Container. Coolify/Docker-kompatibel.

```bash
docker build -t plankton .
docker run -p 3000:3000 -v plankton-data:/app/data plankton
```

Wichtig: `data/` Volume mounten für persistente Daten (Projekte, User, OAuth, Tokens).

## Lizenz

MIT
