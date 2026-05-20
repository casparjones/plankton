# Daten-Storage & Datenmodelle

## Storage-Backends

Plankton unterstützt zwei austauschbare Backends via `DataStore`-Enum.

### FileStore (Standard)

Aktiv wenn `COUCHDB_URI` nicht gesetzt. Speichert jedes Dokument als eigene JSON-Datei.

```
data/
├── projects/<uuid>.json          # ProjectDoc mit allen Tasks
├── users/<uuid>.json             # AuthUser (System-Login)
├── tokens/<uuid>.json            # AgentToken (plk_-Präfix)
└── oauth/
    ├── codes/<hex>.json          # Einmalige Authorization Codes
    ├── clients/<client-id>.json  # Registrierte OAuth-Clients
    └── refresh/<hex>.json        # OAuth Refresh Tokens
```

**Revisions-System:** Einfacher Integer-Counter als String, simuliert CouchDB `_rev`. PUT prüft Rev-Übereinstimmung (optimistisches Locking).

### CouchDB

Aktiv wenn `COUCHDB_URI=http://[user:pass@]host:port` gesetzt.

- Datenbank: `plankton` (via `COUCHDB_DB` konfigurierbar)
- Automatische DB-Erstellung beim Startup (`ensure_db()`)
- Standard CouchDB `_rev`-Mechanismus für Konflikt-Erkennung
- Operationen: `POST`, `GET`, `PUT`, `DELETE`, `_all_docs?include_docs=true`

---

## Datenmodelle

### ProjectDoc (Top-Level-Dokument)

Ein Projekt ist ein flaches JSON-Dokument, das Spalten, Tasks und Team-Mitglieder enthält.

```rust
pub struct ProjectDoc {
    pub id: String,         // UUID
    pub rev: Option<String>,
    pub title: String,
    pub slug: String,       // URL-freundlich, eindeutig (z.B. "mein-projekt")
    pub owner: Option<String>,
    pub columns: Vec<Column>,
    pub users: Vec<User>,
    pub tasks: Vec<Task>,
    pub git: Option<GitConfig>,
}
```

### Column

```rust
pub struct Column {
    pub id: String,
    pub title: String,
    pub slug: String,    // z.B. "TODO", "IN_PROGRESS", "DONE"
    pub order: i32,
    pub color: String,   // Hex-Code (#90CAF9)
    pub hidden: bool,    // true für _archive-Spalte
    pub locked: bool,    // true = nicht löschbar
}
```

Standard-Spalten bei Projekt-Erstellung: `Todo`, `In Progress`, `Testing`, `Done`.

### Task

```rust
pub struct Task {
    pub id: String,
    pub slug: String,           // Auto-generiert aus Titel
    pub title: String,
    pub description: String,
    pub column_id: String,
    pub column_slug: String,    // Für Import/Export
    pub assignee_ids: Vec<String>,
    pub labels: Vec<String>,
    pub order: i32,             // Position innerhalb der Spalte
    pub points: i32,            // Story Points (0–100)
    pub worker: String,         // Primärer Bearbeiter (Name)
    pub creator: String,
    pub logs: Vec<serde_json::Value>,      // Audit-Trail
    pub comments: Vec<serde_json::Value>,  // Diskussionen
    pub created_at: String,     // RFC3339
    pub updated_at: String,
    pub task_type: String,      // "task" | "epic" | "job"
    pub blocks: Vec<String>,    // IDs von Tasks, die dieser blockiert
    pub blocked_by: Vec<String>,
    pub parent_id: String,      // Parent-Epic-ID
    pub subtask_ids: Vec<String>,
}
```

#### Task-Typen

| Typ | Beschreibung |
|-----|-------------|
| `task` | Normale Aufgabe |
| `epic` | Großes Feature mit Subtasks (`subtask_ids`) |
| `job` | Vereinbarte Arbeit / Auftrag |

#### Task-Lifecycle

```
Todo → In Progress → Testing → Done
                                 └── (nach 14 Tagen) → _archive (hidden)
```

### User (Team-Mitglied pro Projekt)

```rust
pub struct User {
    pub id: String,
    pub name: String,
    pub avatar: String,  // URL oder Initialen
    pub role: String,
}
```

### AuthUser (System-Benutzer)

```rust
pub struct AuthUser {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub password_hash: String,  // Argon2id PHC-Format
    pub role: String,
    pub created_at: String,
    pub updated_at: String,
    pub active: bool,
}
```

#### Rollen

| Rolle | Berechtigungen |
|-------|---------------|
| `admin` | Vollzugriff inkl. Admin-Panel, alle MCP-Tools |
| `manager` | Projekt-Management, Approvals, alle Task-Operationen |
| `developer` | Task-Updates, Self-Assignment, Submit for Review |
| `tester` | Review Queue, Approve/Reject |
| `user` | OAuth-Login, alle lesenden MCP-Tools |

### AgentToken

```rust
pub struct AgentToken {
    pub id: String,
    pub name: String,
    pub token: String,   // Format: plk_<48-char-hex>
    pub role: String,
    pub active: bool,
    pub created_at: String,
}
```

---

## Dokument-Hierarchie

```
ProjectDoc
├── Column[]
├── User[]        (Team pro Projekt)
└── Task[]
    ├── logs[]    (Audit-Trail, append-only)
    ├── comments[]
    ├── blocks[] / blocked_by[]   (Task-Relationen)
    └── subtask_ids[] / parent_id (Epic-Hierarchie)

AuthUser          (System-Login, global)
AgentToken        (API-Zugriff für Agenten)
OAuthClient       (Externe Apps)
OAuthAuthCode     (Einmalig, kurzlebig)
OAuthRefreshToken
```
