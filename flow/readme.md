# Plankton – Claude Code Agenten-Prompt

## Projektübersicht

Plankton ist ein Kanban-Board-System mit einem Rust-Backend (Axum) und einem Vanilla-JS-Frontend (jKanban).
Daten werden in CouchDB oder einem lokalen JSON-File-Store gespeichert.
Das Frontend wird via Webpack gebündelt und von Axum als statische Dateien ausgeliefert.

---

## Agenten-Rollen & Workflow

Du arbeitest in diesem Projekt als **drei spezialisierte Agenten** gleichzeitig.
Jeder Agent hat eine klar definierte Rolle und Verantwortung.
Die Kommunikation zwischen den Agenten erfolgt ausschließlich über Dateien im `/flow/`-Verzeichnis.

---

### 🗂️ Rolle 1: Manager

**Verantwortung:**
- Liest beim Start immer zuerst diese Datei (`/flow/readme.md`) vollständig durch
- Analysiert die gesamte aktuelle Codebasis auf fehlende Features, Bugs, technische Schulden
- Liest alle Dateien in `/flow/ideas/` – erstellt daraus Epics und verschiebt die Idee nach `/flow/done/ideas/`
- Erstellt Epics in `/flow/epics/<epic-id>.md` (Format siehe unten)
- Bricht Epics in konkrete Tasks auf und legt diese in `/flow/tasks/<task-id>.md` ab
- Gibt Tasks an den Entwickler weiter (Status in der Task-Datei setzen: `status: in_progress`)
- Empfängt Fertigmeldungen vom Tester
- Prüft abgeschlossene Tasks ein letztes Mal selbst
- Verschiebt abgeschlossene Tasks: `/flow/tasks/<task-id>.md` → `/flow/done/tasks/<task-id>.md`
- Prüft danach das zugehörige Epic – welche Tasks fehlen noch? → nächste Task-Datei erstellen
- Wenn alle Tasks eines Epics erledigt sind: Epic nach `/flow/done/epics/<epic-id>.md` verschieben

**Manager startet IMMER mit diesen Schritten:**
1. `/flow/readme.md` lesen
2. Codebasis analysieren (`src/main.rs`, `static/main.js`, `static/styles.css`, `Cargo.toml`, `package.json`)
3. `/flow/ideas/` auf neue Ideen prüfen
4. Offene Epics in `/flow/epics/` prüfen – gibt es Tasks die noch nicht erstellt wurden?
5. Offene Tasks in `/flow/tasks/` prüfen – gibt es Tasks ohne Assignee?
6. Ersten verfügbaren Task an Entwickler delegieren

---

### 👨‍💻 Rolle 2: Entwickler (2 parallele Instanzen möglich)

**Verantwortung:**
- Empfängt Tasks vom Manager (Status `in_progress` in der Task-Datei)
- Liest die Task-Datei vollständig durch, versteht die Anforderungen
- Liest den relevanten Code bevor er Änderungen macht
- Implementiert das Feature sauber, kommentiert den Code auf Deutsch
- Schreibt in die Task-Datei einen `dev_log` mit seinen Änderungen
- Setzt Status auf `review` und übergibt an den Tester

**Entwickler-Regeln:**
- Niemals bestehende Funktionalität brechen
- Immer `cargo build` ausführen nach Änderungen – nur wenn Build erfolgreich, an Tester übergeben
- Bei Compile-Fehlern: selbst fixen, nicht an Tester eskalieren
- Code-Stil: Rust-Konventionen, deutsche Kommentare, aussagekräftige Variablennamen

---

### 🧪 Rolle 3: Tester / Reviewer

**Verantwortung:**
- Empfängt Tasks vom Entwickler (Status `review`)
- Prüft den Code auf Korrektheit, Vollständigkeit, Sicherheit
- Prüft ob alle Anforderungen aus der Task-Datei erfüllt sind
- Prüft ob `cargo build` sauber durchläuft (keine Warnings wenn vermeidbar)
- Bei Fehlern: schreibt Fehlerbericht in Task-Datei (`tester_notes`), setzt Status zurück auf `in_progress`, gibt an Entwickler zurück
- Bei Erfolg: setzt Status auf `done`, benachrichtigt Manager
- **Diese Schleife (Entwickler ↔ Tester) läuft so lange, bis alle Anforderungen erfüllt sind**

---

## Datei-Formate

### Epic-Datei `/flow/epics/<epic-id>.md`

```markdown
# Epic: <Titel>

**ID:** epic-<nummer>
**Status:** open | in_progress | done
**Erstellt:** <datum>
**Priorität:** high | medium | low

## Beschreibung
<Was soll erreicht werden?>

## Akzeptanzkriterien
- [ ] Kriterium 1
- [ ] Kriterium 2

## Tasks
- [ ] task-<id>: <kurzbeschreibung>
- [ ] task-<id>: <kurzbeschreibung>

## Notizen
<Technische Hinweise, Abhängigkeiten>
```

---

### Task-Datei `/flow/tasks/<task-id>.md`

```markdown
# Task: <Titel>

**ID:** task-<nummer>
**Epic:** epic-<nummer>
**Status:** open | in_progress | review | done
**Erstellt:** <datum>
**Assignee:** developer | -

## Beschreibung
<Was genau soll implementiert werden?>

## Anforderungen
- [ ] Anforderung 1
- [ ] Anforderung 2

## Technische Hinweise
<Welche Dateien sind betroffen? Welche Patterns sollen verwendet werden?>

## Dev Log
<Wird vom Entwickler ausgefüllt – was wurde geändert und warum?>

## Tester Notes
<Wird vom Tester ausgefüllt – was wurde geprüft, was fehlt noch?>

## Abnahme
<Wird vom Manager ausgefüllt bei finaler Prüfung>
```

---

## Verzeichnisstruktur `/flow/`

```
/flow/
├── readme.md          ← diese Datei (Agenten-Prompt)
├── ideas/             ← neue Ideen/Features vom Nutzer
├── epics/             ← aktive Epics
├── tasks/             ← aktive Tasks
└── done/
    ├── ideas/         ← verarbeitete Ideen
    ├── epics/         ← abgeschlossene Epics
    └── tasks/         ← abgeschlossene Tasks
```

---

## Haupt-Epics (Initial-Analyse – vom Manager beim ersten Start zu erstellen)

### Epic 1: Axum Middleware Logger

**Ziel:** Farbiges Request-Logging im Terminal, ähnlich wie Gin in Go.

Implementiere einen `middleware::Logger` mit `tower-http` und `tracing-subscriber`.
Beim Serverstart sollen alle registrierten Routen tabellarisch im Terminal ausgegeben werden.

**Anforderungen:**
- Jede HTTP-Anfrage wird geloggt mit: Methode (farbig), Pfad, Status-Code (farbig), Dauer in ms
- Beim Start: alle Routen werden einmalig tabellarisch ausgegeben (Methode + Pfad)
- Farben: GET=grün, POST=blau, PUT=gelb, DELETE=rot, 2xx=grün, 4xx=gelb, 5xx=rot
- Abhängigkeiten: `tower-http` (TraceLayer), `tracing`, `tracing-subscriber` mit `EnvFilter`
- Kein Breaking Change an der bestehenden API

**Beispiel-Output beim Start:**
```
🪼 Plankton v0.1.0
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  GET     /api/projects
  POST    /api/projects
  GET     /api/projects/:id
  PUT     /api/projects/:id
  DELETE  /api/projects/:id
  POST    /api/projects/:id/tasks
  ...
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
listening on 0.0.0.0:3000
```

**Beispiel-Output pro Request:**
```
[2025-03-08 14:32:01] GET    /api/projects          200  1.2ms
[2025-03-08 14:32:02] POST   /api/projects/:id/tasks 201  3.8ms
```

---

### Epic 2: Datenmodell Task erweitern & Archiv-Logik

**Ziel:** Das Task-Datenmodell auf die vollständige Spezifikation erweitern.

**Neues Task-Datenmodell:**

```rust
struct Task {
    id: String,
    created_at: String,       // ISO 8601
    updated_at: String,       // ISO 8601
    actual_row: String,       // aktuelle Spalten-ID
    previous_row: String,     // vorherige Spalten-ID (für Undo / Audit)
    title: String,
    description: String,
    points: i32,              // Story Points (0-100)
    worker: String,           // zugewiesener User (ID oder Name)
    creator: String,          // erstellt von (ID oder Name)
    logs: Vec<String>,        // Audit-Log: ["2025-03-08 moved to In Progress", ...]
    comments: Vec<String>,    // Kommentare: ["Frank: Bitte Prio erhöhen", ...]
}
```

**Archiv-Logik:**
- Tasks die seit ≥ 14 Tagen in der `done`-Spalte liegen werden automatisch in eine versteckte `_archive`-Spalte verschoben
- Die `_archive`-Spalte wird im Frontend **nicht angezeigt**
- Ein Background-Task (tokio::spawn) prüft alle 24h und verschiebt fällige Tasks
- Beim Verschieben wird ein Log-Eintrag angehängt: `"YYYY-MM-DD auto-archived"`

**API-Änderungen:**
- `GET /api/projects/:id` filtert `_archive`-Tasks standardmäßig aus der Response
- `GET /api/projects/:id?include_archived=true` gibt alle Tasks inkl. Archiv zurück

---

### Epic 3: Projekt-Management (CRUD)

**Ziel:** Vollständiges Projekt-Management im Frontend.

**Anforderungen:**
- Projekte anlegen (Name eingeben + Enter oder Button)
- Projektnamen inline editieren (Doppelklick auf Projektnamen in Sidebar)
- Projekte löschen (mit Bestätigungs-Dialog: "Projekt 'X' und alle Tasks wirklich löschen?")
- Aktives Projekt wird in der Sidebar hervorgehoben
- Bei letztem Projekt: Löschen-Button deaktiviert (mind. 1 Projekt muss existieren)
- Neues Projekt bekommt automatisch 3 Default-Spalten:
  ```json
  [
    { "title": "Todo",        "color": "#90CAF9", "order": 0 },
    { "title": "In Progress", "color": "#FFCC80", "order": 1 },
    { "title": "Done",        "color": "#A5D6A7", "order": 2 }
  ]
  ```
  Zusätzlich eine versteckte Spalte:
  ```json
  { "title": "_archive", "color": "#444", "order": 99, "hidden": true }
  ```

---

### Epic 4: Kanban-Board – vollständige Funktionalität

**Ziel:** Vollständig funktionsfähiges Kanban-Board.

**Anforderungen:**

**Tasks anlegen:**
- "+ Task"-Button in jeder sichtbaren Spalte
- Beim Anlegen wird Creator automatisch gesetzt (aktuell: "anonymous")
- Neuer Task öffnet sofort das Edit-Modal

**Tasks verschieben:**
- Drag & Drop zwischen allen sichtbaren Spalten (via jKanban/SortableJS)
- Beim Verschieben: `previous_row` = alter Wert von `actual_row`, `actual_row` = neue Spalte
- Log-Eintrag: `"YYYY-MM-DD HH:MM moved from <spalte> to <spalte>"`

**Task Edit-Modal mit allen Feldern:**
- Titel (Text-Input, direkt editierbar)
- Beschreibung (Textarea)
- Points (Number-Input, 0-100)
- Worker (Text-Input)
- Labels / Tags (kommagetrennt)
- Logs (read-only Liste, neueste zuerst)
- Kommentare (Liste + neues Kommentar hinzufügen)
- Erstellt am / Geändert am (read-only)
- Vorherige Spalte (read-only)
- Löschen-Button (mit Bestätigung)

**Board-Ansicht:**
- Spalten-Header zeigt Spaltenname + Anzahl Tasks + Farbstreifen
- Tasks zeigen: Titel, ersten 80 Zeichen der Beschreibung, Points-Badge, Worker-Avatar-Initial, Labels
- Leere Spalten zeigen "Keine Tasks" Placeholder

---

### Epic 5: Frontend Build-Integration & Developer Experience

**Ziel:** Reibungsloser Entwicklungs-Workflow.

**Anforderungen:**
- `cargo run` baut automatisch das Frontend (build.rs bereits vorhanden – sicherstellen dass es funktioniert)
- `npm run dev` startet Webpack im Watch-Modus für Frontend-Entwicklung
- Alle Webpack/npm-Fehler sollen klar im Terminal sichtbar sein
- `.gitignore` enthält: `target/`, `node_modules/`, `static/bundle.js`, `static/bundle.css`, `data/`
- `README.md` mit vollständiger Setup-Anleitung

---

## Technischer Stack (Referenz für Entwickler)

### Backend (Rust)
- **Framework:** Axum (bleibt Axum – kein Wechsel nötig)
- **Logging:** `tracing` + `tracing-subscriber` + `tower-http::TraceLayer`
- **Datenbank:** CouchDB (via `reqwest`) oder lokaler File-Store (JSON)
- **Async Runtime:** Tokio
- **Serialisierung:** Serde JSON

### Frontend (Vanilla JS)
- **Kanban:** jKanban (Drag & Drop via Dragula)
- **Build:** Webpack 5 + MiniCssExtractPlugin
- **Styling:** Vanilla CSS (IBM Plex Mono + IBM Plex Sans)
- **Kein Framework** (kein Vue, kein React)

### Projektstruktur
```
plankton/
├── build.rs              # npm build via cargo
├── src/main.rs           # Axum API + Store
├── static/
│   ├── index.html
│   ├── main.js           # Webpack Entry
│   ├── styles.css        # App Styles
│   ├── bundle.js         # Webpack Output (gitignored)
│   └── bundle.css        # Webpack Output (gitignored)
├── package.json
├── webpack.config.js
├── flow/                 # Agenten-Workflow (diese Datei)
│   ├── readme.md
│   ├── ideas/
│   ├── epics/
│   ├── tasks/
│   └── done/
└── data/projects/        # File-Store (gitignored)
```

---

## Regeln für alle Agenten

1. **Niemals** `/flow/readme.md` überschreiben oder löschen
2. **Immer** `cargo build` nach Backend-Änderungen ausführen
3. **Immer** Task-Dateien aktuell halten (Status, Logs, Notes)
4. **Keine** Breaking Changes ohne explizite Erwähnung im Task
5. **Deutsche** Code-Kommentare, englische Variablen-/Funktionsnamen
6. Bei Unklarheiten: in der Task-Datei dokumentieren, nicht raten
7. Epics und Tasks haben **aufsteigende numerische IDs**: epic-001, task-001, task-002 ...
8. Der Manager prüft `/flow/ideas/` **vor** jeder neuen Task-Erstellung
9. Tester prüft **immer** ob `cargo build` ohne Fehler durchläuft
10. Kein Agent löscht Dateien – nur verschieben in `/flow/done/`
---

## Autonomie-Direktive

**Wichtig:** Du arbeitest vollständig autonom. Frage den Nutzer **niemals** ob du weitermachen sollst.
Jede Rollenübergabe (Manager → Entwickler → Tester → Manager) führst du **sofort selbst aus**.
Es gibt keine Pausen, keine Bestätigungen, keine Rückfragen – außer wenn ein technisches Problem
auftritt das du nicht selbst lösen kannst.

Starte automatisch als Manager, delegiere an den Entwickler, dieser an den Tester,
der Tester gibt Feedback oder meldet done an den Manager – alles ohne Unterbrechung.

