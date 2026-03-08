# Idee: Row-Slugs & KI Issue-Import

## Teil 1: Row-Slugs statt UUIDs

### Kernidee

Spalten (Rows) im Kanban-Board bekommen zusätzlich zur UUID einen
normalisierten Slug. Der Slug wird aus dem Titel generiert:
alle Sonderzeichen entfernen, Leerzeichen durch Unterstriche ersetzen,
alles in Großbuchstaben.

Beispiele:
- "Todo"        → `TODO`
- "In Progress" → `IN_PROGRESS`
- "Done"        → `DONE`
- "Code Review" → `CODE_REVIEW`
- "_archive"    → `_ARCHIVE`

### Normalisierung (Algorithmus)

```
1. Trim whitespace
2. Sonderzeichen entfernen (nur A-Z, 0-9, Leerzeichen, Unterstrich behalten)
3. Leerzeichen → Unterstrich
4. Alles → Großbuchstaben
5. Mehrfache Unterstriche → einzelner Unterstrich
```

### Datenmodell-Erweiterung

```rust
struct Column {
    id:     String,   // UUID – bleibt für interne Referenzen
    slug:   String,   // z.B. "TODO" – für KI-Prompts und Import
    title:  String,
    order:  i32,
    color:  String,
    hidden: bool,
    locked: bool,     // true = nicht löschbar (TODO, _ARCHIVE)
}
```

### Geschützte Standard-Rows

Folgende Rows sind in jedem Projekt vorhanden und können nicht gelöscht werden
(`locked: true`):

- `TODO`     – Eingangs-Row für alle neuen Tasks (auch KI-generierte)
- `_ARCHIVE` – Versteckte Archiv-Row (seit 14 Tagen in DONE)

Der Lösch-Button ist für locked Rows deaktiviert und zeigt einen Tooltip:
"Diese Spalte kann nicht gelöscht werden"

### Slug in der API

- `GET /api/projects/:id` gibt den Slug mit zurück
- Tasks können per Slug referenziert werden:
  `POST /api/projects/:id/tasks` mit `column_slug: "TODO"` statt `column_id`
- Backend löst Slug → UUID auf bevor der Task gespeichert wird
- Slug muss pro Projekt eindeutig sein

### Prompt-Integration

KI-Agenten können in ihren generierten Tasks einfach schreiben:
```json
{ "column_slug": "TODO" }
```
Statt die UUID der Spalte kennen zu müssen.

---

## Teil 2: KI Issue-Import

### Kernidee

Ein "Import Issues" Button im Board-Header öffnet einen Import-Dialog.
Der Nutzer kann dort eine JSON-Liste von Tasks einfügen (z.B. von einer KI generiert)
oder eine JSON-Datei hochladen. Die App validiert jeden Task, zeigt
Fehler/Warnings an und importiert die validen Tasks direkt ins Projekt.

Kein manuelles Kopieren in die Projekt-JSON mehr.

### Import-Dialog (Frontend)

1. Button "Import Issues" im Board-Header
2. Modal öffnet sich mit:
    - Textarea für JSON-Paste
    - oder: Datei-Upload (.json)
    - "Validieren" Button
3. Nach Validierung: Ergebnis-Tabelle mit einer Zeile pro Task:
    - ✅ Grün: Task ist valide, wird importiert
    - ⚠️ Gelb: Warning – Feld fehlt oder wurde automatisch gesetzt (z.B. creator)
    - ❌ Rot: Fehler – Task kann nicht importiert werden (Pflichtfeld fehlt)
4. Zusammenfassung: "12 Tasks valide, 2 Warnings, 1 Fehler"
5. Button "Import starten" importiert alle validen Tasks (auch die mit Warnings)
6. Fehlerhafte Tasks werden übersprungen und in einer Liste angezeigt

### Erwartetes JSON-Format (KI-Output)

```json
[
  {
    "title": "Login-Seite implementieren",
    "description": "Username + Password Formular bauen",
    "column_slug": "TODO",
    "points": 3,
    "worker": "",
    "creator": "",
    "labels": ["auth", "frontend"],
    "comments": [],
    "logs": []
  }
]
```

### Validierungsregeln

**Fehler (❌) – Task wird nicht importiert:**
- `title` fehlt oder ist leer
- `column_slug` existiert nicht im Projekt
- `points` ist kein Integer oder außerhalb 0-100

**Warnings (⚠️) – Task wird importiert, Feld wird automatisch gesetzt:**
- `creator` ist leer → wird auf den aktuell eingeloggten Nutzer gesetzt
- `column_slug` fehlt → Task landet automatisch in `TODO`
- `labels` fehlt → wird als leeres Array gesetzt
- `comments` fehlt → wird als leeres Array gesetzt
- `logs` fehlt → wird als leeres Array gesetzt
- `points` fehlt → wird auf 0 gesetzt
- `worker` fehlt → bleibt leer (kein Warning, optionales Feld)

**Automatisch gesetzt (immer, kein Warning):**
- `id` → neue UUID wird generiert
- `created_at` → aktueller Timestamp
- `updated_at` → aktueller Timestamp
- `actual_row` → wird aus `column_slug` aufgelöst
- `previous_row` → leer

### Backend-Endpunkt

```
POST /api/projects/:id/import
Body: { "tasks": [...] }
Response: {
  "imported": 12,
  "warnings": [...],
  "errors": [...],
  "skipped": 1
}
```

Der Endpunkt validiert serverseitig nochmal (Frontend-Validierung ist nur UX).

### Log-Eintrag beim Import

Jeder importierte Task bekommt automatisch einen Log-Eintrag:
`"YYYY-MM-DD HH:MM imported via Issue Import [Frank]"`

### KI-Prompt Snippet (für die /flow/readme.md)

Damit KI-Agenten wissen wie sie Tasks für den Import formatieren sollen,
wird in die /flow/readme.md ein Abschnitt "Task-Format für Issue-Import"
eingefügt:

```
Wenn du Tasks für Plankton generierst, nutze dieses JSON-Format:
[
  {
    "title": "Pflichtfeld – kurzer Titel",
    "description": "Ausführliche Beschreibung",
    "column_slug": "TODO",
    "points": 0-100,
    "worker": "",
    "creator": "",
    "labels": ["label1", "label2"],
    "comments": [],
    "logs": []
  }
]
Gib nur das JSON-Array zurück, keinen weiteren Text.
```