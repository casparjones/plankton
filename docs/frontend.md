# Frontend-Architektur

## Stack

| | |
|-|-|
| Framework | Vue 3 (Composition API) |
| Sprache | TypeScript (strict) |
| Build | Webpack (via `npm run build` / `npm run dev`) |
| Styling | Tailwind CSS v4 + shadcn-vue-Komponenten |
| Drag & Drop | vue-draggable-plus (SortableJS) |
| Toast | vue-toastification |
| Icons | lucide-vue-next |

Das Build-Skript (`build.rs`) triggert `npm run build` automatisch beim `cargo build`, falls kein Bundle vorhanden ist.

---

## Verzeichnisstruktur

```
src/frontend/
├── main.ts               # App-Mount, globale Initialisierung
├── App.vue               # Root-Komponente: Router, Header, Theme-Toggle
├── api.ts                # HTTP-Client (get/post/put/del)
├── state.ts              # Reaktiver globaler State
├── types/index.ts        # TypeScript-Interfaces
├── utils.ts              # Hilfsfunktionen (Slug, Datum, etc.)
├── toast.ts              # Toast-Wrapper
├── components/
│   ├── KanbanBoard.vue   # Board mit Spalten und Drag-and-Drop
│   ├── TaskModal.vue     # Erstellen/Bearbeiten-Popup
│   ├── TaskDetail.vue    # Vollständige Task-Ansicht
│   ├── ArchivePanel.vue  # Archivierte Tasks + Restore
│   ├── ImportPage.vue    # CSV/JSON-Bulk-Import
│   ├── auth.ts           # Login-Komponente
│   ├── board.ts          # Board-Hilfsfunktionen
│   └── admin.ts          # Admin-Panel
├── services/
│   ├── sse-service.ts    # SSE-Verbindung und Event-Handler
│   └── project-service.ts# Laden und Cachen von Projekten
├── composables/          # Vue 3 Composition-API-Hooks
├── i18n/                 # Übersetzungen (de, en)
├── styles/globals.css    # Globale Styles
└── index.html            # HTML-Template für Webpack
```

---

## State Management

Kein Vuex/Pinia — stattdessen ein zentrales reaktives Objekt:

```typescript
export const state: AppState = reactive({
  projects: ProjectDoc[],       // Alle geladenen Projekte
  project: ProjectDoc | null,   // Aktives Projekt
  kanban: any | null,           // Aufbereitete Board-Daten
  editingTask: Task | null,     // Task im Bearbeitungs-Popup
  isNewTask: boolean,
  selectedTasks: Set<string>,   // Bulk-Selektion
  eventSource: EventSource | null,
  currentUser: AuthUser | null,
  isDragging: boolean,          // Sperrt SSE-Updates während Drag
  detailTask: Task | null,      // Task in der Detailansicht
  allUsers: User[],
})
```

---

## HTTP-Client (`api.ts`)

```typescript
const api = {
  get<T>(path: string): Promise<T>,
  post<T>(path: string, body: unknown): Promise<T>,
  put<T>(path: string, body: unknown): Promise<T>,
  del(path: string): Promise<void>,
}
```

- 401 → Automatischer Redirect zur Login-Seite
- Fehler-Format: `{"error": "...", "code": "OPTIONAL_UPPERCASE_CODE"}`

---

## Routing

Client-seitiges SPA-Routing:

| Pfad | Komponente |
|------|-----------|
| `/` | Home / Login |
| `/p/:slug` | KanbanBoard (slug oder UUID) |
| `/import` | ImportPage |

Der Server liefert für `/p/*` und `/import` immer `index.html` zurück (SPA-Fallback).

---

## Echtzeit-Updates (`sse-service.ts`)

Verbindet sich mit `GET /api/projects/:id/events` (EventSource).

Empfangene Events werden direkt in den State gepatcht:

| Event | Aktion |
|-------|--------|
| `task_created` | Task zum State hinzufügen |
| `task_updated` | Task-Felder im State patchen |
| `task_moved` | Task in andere Spalte verschieben |
| `task_deleted` | Task aus State entfernen |
| `project_update` | Full-Refetch |

Während Drag-and-Drop (`state.isDragging = true`) werden eingehende SSE-Updates ignoriert, um Konflikte zu vermeiden.

Siehe [realtime-sse.md](realtime-sse.md) für vollständige Event-Dokumentation.

---

## Wichtige Komponenten

### KanbanBoard.vue
- Zeigt alle Spalten des aktiven Projekts
- Drag-and-Drop via vue-draggable-plus
- Optimistisches UI-Update: State wird sofort gepatcht, API-Call läuft im Hintergrund

### TaskModal.vue
- Öffnet sich beim Erstellen oder Klick auf eine Task
- Felder: Titel, Beschreibung, Labels, Story Points, Worker, Task-Typ
- Schließt sich bei Escape oder Klick außerhalb

### TaskDetail.vue
- Vollständige Ansicht: Logs, Comments, Relationen, Subtasks
- Inline-Comment-Eingabe
- Board-Info-Copy-Button (für KI-Sharing)

### ArchivePanel.vue
- Side-Panel für archivierte Tasks
- Zeigt Tasks aus der versteckten `_archive`-Spalte
- Restore-Button verschiebt Task zurück in „Todo"

### ImportPage.vue
- Tabellen-Preview des zu importierenden Datensatzes
- Spalten-Mapping (CSV-Header → Plankton-Felder)
- Fehler-Reporting pro Zeile
