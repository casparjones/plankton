# Idee: Refactoring – Backend & Frontend Struktur

## Kernidee

Kein neues Feature – reines Refactoring.
Die `main.rs` ist bereits über 2000 Zeilen groß und enthält alles in einer Datei:
Routing, Handler, Business-Logik, Datenmodelle, Store-Implementierungen.
Die `main.js` ist ebenfalls sehr groß und unstrukturiert.
Beide Dateien sollen in eine saubere, wartbare Struktur aufgeteilt werden.

Ziel: Jede Datei hat eine einzige Verantwortung. Neue Features können
danach isoliert in die richtige Schicht eingebaut werden ohne die gesamte
Codebasis zu verstehen.

---

## Backend – Rust Struktur

### Ziel-Verzeichnisstruktur

```
src/
├── main.rs                  # Nur: Server-Start, Router-Aufbau, State-Initialisierung
├── config.rs                # Umgebungsvariablen, Port, DB-URL etc.
├── error.rs                 # ApiError enum + IntoResponse impl
├── state.rs                 # AppState struct
│
├── models/
│   ├── mod.rs
│   ├── project.rs           # ProjectDoc, Column, Task, User structs + Serde
│   └── requests.rs          # DeleteQuery, MoveTaskRequest, McpCall etc.
│
├── controllers/
│   ├── mod.rs
│   ├── project_controller.rs  # Handler: list, create, get, update, delete project
│   ├── task_controller.rs     # Handler: create, update, delete, move task
│   ├── column_controller.rs   # Handler: create, update, delete column
│   ├── user_controller.rs     # Handler: create, update, delete user
│   ├── event_controller.rs    # Handler: SSE project_events
│   └── mcp_controller.rs      # Handler: list_tools, call_tool
│
├── services/
│   ├── mod.rs
│   ├── project_service.rs   # Business-Logik: default_project, publish_update
│   └── mcp_service.rs       # MCP Tool-Ausführung (aus call_tool extrahiert)
│
└── store/
    ├── mod.rs               # DataStore enum + trait-ähnliche Delegation
    ├── couch.rs             # CouchDb struct + alle Methoden
    └── file.rs              # FileStore struct + alle Methoden
```

### Regeln für die Aufteilung

- `main.rs` darf nur noch den Router aufbauen und den Server starten (~50 Zeilen)
- Controller kennen nur den AppState und rufen Services auf
- Services enthalten die Business-Logik und kennen den Store
- Store-Implementierungen kennen keine Controller oder Services
- Models sind reine Datenstrukturen, keine Logik
- `error.rs` wird von allen Schichten importiert, importiert selbst nichts aus dem Projekt
- Keine zirkulären Abhängigkeiten

### Cargo.toml bleibt unverändert – kein neues Crate nötig

---

## Frontend – JavaScript Struktur

### Ziel-Verzeichnisstruktur

```
static/
├── index.html               # Unverändert – nur bundle.css + bundle.js laden

src/frontend/                # Webpack Entry ab hier (webpack.config.js anpassen)
├── main.js                  # Nur: init(), DOMContentLoaded, App zusammenbauen
├── state.js                 # Zentraler App-State, State-Mutations
├── api.js                   # Alle fetch()-Aufrufe (get, post, put, del)
├── router.js                # Einfaches Client-seitiges Routing (falls nötig)
│
├── components/
│   ├── board.js             # Board rendern, jKanban initialisieren + zerstören
│   ├── task-card.js         # taskToItem() – Task HTML generieren
│   ├── task-modal.js        # Modal öffnen, schließen, speichern, löschen
│   ├── sidebar.js           # Projektliste rendern, aktives Projekt markieren
│   └── column-header.js     # Spalten-Header mit Farbe + Add-Button
│
├── services/
│   ├── project-service.js   # loadProjects, openProject, createProject
│   ├── task-service.js      # createTask, saveTask, deleteTask, moveTask
│   └── sse-service.js       # SSE-Verbindung verwalten (subscribeSSE)
│
└── styles/
    ├── main.css             # CSS-Einstiegspunkt – importiert alle anderen
    ├── layout.css           # App-Layout, Sidebar, Main
    ├── board.css            # jKanban Overrides, Spalten, Board-Container
    ├── task.css             # Task-Karten, Labels, Avatare
    ├── modal.css            # Modal-Overlay, Modal-Inhalt
    └── variables.css        # CSS Custom Properties (:root Variablen, Fonts)
```

### webpack.config.js anpassen

Entry-Point von `./static/main.js` auf `./src/frontend/main.js` ändern.
Output bleibt `static/bundle.js` + `static/bundle.css`.

### build.rs anpassen

`rerun-if-changed` Pfade von `static/main.js` auf `src/frontend/**` aktualisieren.

---

## Regeln für das Refactoring

1. **Kein Feature wird hinzugefügt** – 1:1 Funktionserhalt, nur Struktur ändert sich
2. **Nach jedem extrahierten Modul:** `cargo build` muss erfolgreich sein
3. **Frontend:** Nach jeder extrahierten Komponente muss `npm run build` erfolgreich sein
4. **Keine API-Änderungen** – alle Endpunkte bleiben identisch
5. **Keine UI-Änderungen** – das Board sieht danach exakt gleich aus
6. Der Tester prüft nach dem Refactoring explizit:
    - Alle API-Endpunkte antworten korrekt
    - Board lädt und zeigt Projekte an
    - Tasks können angelegt, verschoben und gelöscht werden
    - SSE-Events funktionieren
    - `cargo build` produziert keine neuen Warnings