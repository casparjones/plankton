# Echtzeit via Server-Sent Events (SSE)

## Endpunkt

```
GET /api/projects/:id/events
Authorization: Bearer <token>  (oder Cookie)
```

Response: `Content-Type: text/event-stream`

Der Client kann als UUID oder Slug ansprechen. Bei Verbindungsabbruch reconnectet der Browser automatisch.

---

## Event-Format

Alle Events folgen diesem Format:

```
event: project_event
data: {"event": "<type>", "data": {...}}
```

### task_created

Neue Task wurde erstellt (von einem anderen Client oder Agenten).

```json
{
  "event": "task_created",
  "data": {
    "id": "uuid",
    "title": "...",
    "column_id": "...",
    "order": 0
    // ... alle Task-Felder
  }
}
```

### task_updated

Bestehende Task wurde geändert (Titel, Beschreibung, Labels, etc.).

```json
{
  "event": "task_updated",
  "data": {
    "id": "uuid",
    "title": "neuer Titel",
    "updated_at": "2026-01-01T12:00:00Z"
    // ... geänderte Felder
  }
}
```

### task_moved

Task wurde in eine andere Spalte verschoben.

```json
{
  "event": "task_moved",
  "data": {
    "id": "uuid",
    "column_id": "neue-spalten-id",
    "order": 2
  }
}
```

### task_deleted

Task wurde gelöscht.

```json
{
  "event": "task_deleted",
  "data": {
    "task_id": "uuid"
  }
}
```

### project_update

Vollständiges Projekt-Update nötig (Legacy-Fallback). Client sollte danach das gesamte Projekt neu laden.

```json
{
  "event": "project_update",
  "data": {}
}
```

---

## Server-Implementierung

### AppState

```rust
events: Arc<Mutex<HashMap<String, broadcast::Sender<String>>>>
```

Pro Projekt existiert ein `broadcast::Sender`. Jeder SSE-Client erhält einen `Receiver`.

### Publishing (project_service.rs)

```rust
pub async fn publish_event(
    state: &AppState,
    project_id: &str,
    event_type: &str,
    data: serde_json::Value,
)
```

Wird nach jeder schreibenden Operation aufgerufen:
- Nach `create_task`, `update_task`, `move_task`, `delete_task`
- Löst Slug → UUID auf falls nötig
- Sendet JSON-serialisierten Event an alle Subscriber

### Heartbeat

Bei längerer Inaktivität sendet der Server:

```
event: heartbeat
data: ping
```

Verhindert Timeout durch Reverse Proxies.

---

## Frontend-Integration

### Verbindungsaufbau (`sse-service.ts`)

```typescript
const source = new EventSource(`/api/projects/${projectId}/events`)

source.addEventListener('project_event', (e) => {
  const { event, data } = JSON.parse(e.data)
  handleEvent(event, data)
})
```

### Drag-and-Drop-Guard

Während ein Nutzer eine Task zieht (`state.isDragging = true`), werden eingehende SSE-Events gepuffert oder ignoriert, um Konflikte zu vermeiden. Nach dem Drop wird der State mit dem Server abgeglichen.

### Auto-Reconnect

`EventSource` reconnectet automatisch bei Netzwerkfehlern. Die Verbindung wird beim Verlassen des Boards geschlossen.
