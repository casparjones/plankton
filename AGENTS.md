# AGENTS.md — plankton

Kanban board + CLI für KI-Agenten-Workflows. MCP/JSON-RPC API + Rust-Backend + Vue-Frontend.

## Commands

- Build Backend: `cargo build --release`
- Build Frontend: `npm run build` (Webpack → `static/`)
- Build Docker: `docker build -t plankton .`
- Test: `cargo test`
- Lint: `cargo clippy -- -D warnings` + `cargo fmt --check`
- Run (lokal): `cargo run` → http://localhost:3000
- Run (mit CouchDB): `COUCHDB_URI=http://admin:password@localhost:5984 cargo run`
- Frontend Dev: `npm run dev` (webpack watch, parallel zu `cargo run`)

> Default-Credentials beim ersten lokalen Start: `admin` / `admin` — nur für lokale Dev-Umgebung, **vor jedem Public-Deployment ändern**.

## Dokumentation

Architekturdokumentation liegt in `docs/`. Vor größeren Änderungen lesen:

| Datei | Inhalt |
|-------|--------|
| [docs/architecture-overview.md](docs/architecture-overview.md) | Gesamtarchitektur, Verzeichnisstruktur, Request-Lifecycle, Background-Tasks |
| [docs/data-storage.md](docs/data-storage.md) | FileStore vs. CouchDB, alle Datenmodelle (Task, Project, User, Token) |
| [docs/api-reference.md](docs/api-reference.md) | Vollständige REST-API mit allen Endpunkten |
| [docs/mcp-server.md](docs/mcp-server.md) | MCP-Tools, JSON-RPC-Transport, Rollen-Zugriff, Integrations-Beispiele |
| [docs/auth-oauth.md](docs/auth-oauth.md) | JWT, Agent-Tokens, OAuth 2.0 Authorization Code Flow, CLI Device Flow |
| [docs/frontend.md](docs/frontend.md) | Vue 3 State-Management, Komponenten, HTTP-Client, Routing |
| [docs/realtime-sse.md](docs/realtime-sse.md) | SSE-Endpunkt, Event-Typen, Server-Publishing, Frontend-Integration |

## Conventions

- Tests im selben Modul (`#[cfg(test)]`) oder als Integration-Test unter `tests/`
- JSON-RPC-Antworten immer mit klarer Error-Struktur
- TDD bevorzugt: failing test → Implementierung → grüner Test
- Root-Cause-Hypothesen in Tickets/Commits als Spekulation markieren, „validate this assumption" als ersten Schritt

## Boundaries

**Never:** Secrets committen · Tests skippen · Breaking Changes an JSON-RPC-API ohne Versionierung · Bestehende CLI-Subcommands umbenennen (Backwards Compatibility).

**Ask first:** Datenbank-Migration · API-Schema-Änderungen · Neuer Subcommand mit destruktivem Verhalten.

## Local agent setup

Persönliche Agent-Workflows (Ticket-Board, Rollen-Aufteilung etc.) gehören in `AGENTS.local.md` (gitignored).