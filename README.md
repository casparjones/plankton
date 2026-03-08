# 🪼 Plankton

Minimales Kanban-Board mit Rust-Backend (Axum) + jKanban-Frontend.  
Jede Änderung am Board wird sofort als JSON in CouchDB (oder lokal als Datei) gespeichert.

## Voraussetzungen

- Rust (stable) + Cargo
- Node.js ≥ 18 + npm

## Setup & Starten

```bash
# Alles in einem Schritt – Cargo ruft npm automatisch via build.rs auf:
cargo run

# Mit CouchDB:
COUCHDB_URI=http://admin:password@localhost:5984 cargo run

# Anderen Port:
PORT=8080 cargo run
```

Browser: **http://localhost:3000**

## Entwicklung (Frontend Hot-Reload)

```bash
# Terminal 1 – Webpack im Watch-Modus
cd frontend && npm run dev

# Terminal 2 – Rust-Server (ohne build.rs-Frontend-Build)
cargo run
```

## Projektstruktur

```
plankton/
├── build.rs              # Ruft npm build vor cargo build auf
├── src/
│   └── main.rs           # Axum REST-API + SSE + File/CouchDB Store
├── frontend/
│   ├── package.json
│   ├── webpack.config.js
│   └── src/
│       ├── main.js       # jKanban + Vanilla JS App
│       └── style.css     # Dark Industrial Theme
└── static/
    ├── index.html        # Wird von Axum served
    ├── bundle.js         # Webpack Output
    └── bundle.css        # Webpack Output
```

## Datenformat (JSON)

Jedes Projekt ist ein flaches JSON-Dokument:

```json
{
  "_id": "uuid",
  "_rev": "1",
  "title": "Mein Projekt",
  "columns": [{ "id": "uuid", "title": "Todo", "order": 0, "color": "#90CAF9" }],
  "users":   [{ "id": "uuid", "name": "Frank", "avatar": "F", "role": "dev" }],
  "tasks":   [{ "id": "uuid", "title": "Feature X", "column_id": "...", ... }]
}
```

Im File-Store liegt jedes Projekt unter `data/projects/<id>.json` – direkt importierbar/exportierbar.