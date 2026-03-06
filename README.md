# Plankton

Plankton is a CouchDB-backed Kanban board written in Rust (Axum) with a Vue 3 + Pinia + Vuetify frontend.

## Features
- One CouchDB document per project
- REST CRUD APIs for projects, tasks, columns, and users
- Optimistic concurrency with CouchDB `_rev`
- SSE endpoint for live updates (`/api/projects/:id/events`)
- MCP-like tool endpoint for LLM actions (`/mcp/tools`, `/mcp/call`)
- Dark mode, mobile-friendly board layout
- Drag/drop between columns
- Manual JSON import/export via GET/PUT on `/api/projects/:id`

## Run locally

```bash
export COUCHDB_URL=http://127.0.0.1:5984
export COUCHDB_DB=plankton
cargo run
```

Alternativ mit neuer Variable:

```bash
export COUCHDB_URI=http://127.0.0.1:5984
cargo run
```

Wenn `COUCHDB_URI` (oder `COUCHDB_URL`) **nicht** gesetzt ist, nutzt Plankton automatisch ein Dateisystem-Fallback in `./data/projects/*.json`.

Open `http://localhost:3000`.

## MCP tool call example

```bash
curl -X POST http://localhost:3000/mcp/call \
  -H 'content-type: application/json' \
  -d '{"tool":"summarize_board","arguments":{"project_id":"<id>"}}'
```

## Docker

```bash
docker build -t plankton .
docker run -p 3000:3000 -e COUCHDB_URL=http://host.docker.internal:5984 plankton
```
