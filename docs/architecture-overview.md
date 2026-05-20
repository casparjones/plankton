# Architektur-Übersicht

Plankton ist ein kollaboratives Kanban-Board mit MCP-Integration für KI-Agenten-Workflows.

## Stack

| Schicht | Technologie |
|---------|------------|
| Backend | Rust, Axum, Tokio (async) |
| Frontend | Vue 3, TypeScript, Webpack |
| Datenbank | JSON-FileStore (Standard) oder CouchDB |
| Auth | JWT (Cookie/Bearer) + OAuth 2.0 + Device Flow |
| API | REST + JSON-RPC 2.0 (MCP) + SSE |
| Deployment | Docker (Multi-Stage), Single Binary |

## Verzeichnisstruktur

```
plankton/
├── src/
│   ├── main.rs                    # Entry Point, Router, Background-Tasks
│   ├── config.rs                  # Env-Variablen (PORT, JWT_SECRET, COUCHDB_URI)
│   ├── state.rs                   # AppState mit Mutexes für Sessions/Events
│   ├── error.rs                   # ApiError-Enum → HTTP-Responses
│   ├── middleware.rs              # auth_guard, request_logger, startup_banner
│   ├── models/
│   │   ├── project.rs             # ProjectDoc, Column, Task, User, GitConfig
│   │   ├── auth.rs                # AuthUser, Claims, CliSession, OAuthClient, Tokens
│   │   └── requests.rs            # Request/Response-DTOs
│   ├── store/
│   │   ├── mod.rs                 # DataStore-Enum (Couch | File), User/Token-Verwaltung
│   │   ├── file.rs                # FileStore — JSON-Dateien in data/
│   │   └── couch.rs               # CouchDB — HTTP-Wrapper für REST-API
│   ├── services/
│   │   ├── auth_service.rs        # JWT, Argon2-Hashing, Token-Extraktion
│   │   ├── project_service.rs     # SSE-Events, Task-Archivierung, Default-Projekt
│   │   └── git_service.rs         # Git-Sync (deaktiviert)
│   ├── controllers/               # HTTP-Handler (ein Modul pro Ressource)
│   │   ├── auth_controller.rs     # /auth/*
│   │   ├── oauth_controller.rs    # /oauth/*, /authorize, /token
│   │   ├── project_controller.rs  # /api/projects/*
│   │   ├── task_controller.rs     # /api/projects/:id/tasks/*
│   │   ├── column_controller.rs   # /api/projects/:id/columns/*
│   │   ├── user_controller.rs     # /api/projects/:id/users/*
│   │   ├── event_controller.rs    # /api/projects/:id/events (SSE)
│   │   ├── admin_controller.rs    # /api/admin/*
│   │   ├── cli_controller.rs      # /auth/cli-* (Device Flow)
│   │   └── mcp_controller.rs      # /mcp/* (JSON-RPC 2.0)
│   └── frontend/                  # Vue 3 + TypeScript
│       ├── main.ts
│       ├── App.vue
│       ├── api.ts                 # HTTP-Client
│       ├── state.ts               # Reaktiver globaler State
│       ├── types/index.ts
│       ├── components/            # .vue-Komponenten
│       ├── services/              # sse-service.ts, project-service.ts
│       └── composables/           # Vue 3 Composition-API-Hooks
├── static/                        # Build-Output (Webpack → bundle.*.js/css)
├── data/                          # Persistenter FileStore
│   ├── projects/                  # <uuid>.json
│   ├── users/
│   ├── tokens/
│   └── oauth/codes|clients|refresh/
├── build.rs                       # Build-Script (Frontend-Build-Trigger)
├── Dockerfile                     # Multi-Stage: Node → Rust → Debian-Slim
└── docs/                          # Diese Dokumentation
```

## Request-Lifecycle

```
HTTP Request
  └── Axum Router
       ├── request_logger (Middleware)
       ├── auth_guard (Middleware)
       │    ├── Cookie plankton_token=<jwt>
       │    ├── Header Authorization: Bearer <jwt>
       │    └── Header Authorization: Bearer plk_<hex>  ← Agent-Token
       └── Controller-Handler
            ├── AppState (via Axum Extension)
            │    ├── store: DataStore (FileStore | CouchDB)
            │    ├── events: broadcast::Sender pro Projekt
            │    ├── jwt_secret
            │    ├── cli_sessions
            │    ├── mcp_sessions
            │    ├── oauth_clients
            │    ├── oauth_codes
            │    └── oauth_refresh_tokens
            └── HTTP Response (JSON)
```

## Background-Tasks (spawned bei Startup)

| Task | Interval | Aufgabe |
|------|----------|---------|
| Task-Archivierung | täglich | Tasks ≥ 14 Tage in „Done" → Spalte `_archive` |
| CLI-Session-Cleanup | stündlich | Sessions > 5 Min alt löschen |

## Fehlerbehandlung

Alle Fehler laufen durch `ApiError`:

```
ApiError::NotFound    → 404  {"error": "...", "code": "NOT_FOUND"}
ApiError::BadRequest  → 400
ApiError::Conflict    → 409
ApiError::Unauthorized→ 401
ApiError::Forbidden   → 403
ApiError::Request     → 502  (reqwest-Fehler)
ApiError::Io          → 500
ApiError::Json        → 400
```

## Weiterführende Dokumentation

- [Daten-Storage & Datenmodelle](data-storage.md)
- [REST-API-Referenz](api-reference.md)
- [MCP-Server (JSON-RPC)](mcp-server.md)
- [Authentifizierung & OAuth 2.0](auth-oauth.md)
- [Frontend-Architektur](frontend.md)
- [Echtzeit via SSE](realtime-sse.md)
