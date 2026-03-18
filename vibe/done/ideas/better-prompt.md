# Plankton – Claude Code Erweiterung: KI-Agenten-Workflow

## Kontext

Die App hat rechts oben die Möglichkeit, einen Prompt einzugeben.

Diese Funktion wird erweitert: Es gibt zwei Tabs.

**Tab 1 – Simple (tasks as json):** Der Prompt wie er aktuell ist – unverändert um schnell Task in Plankton einzufügen
**Tab 2 – Plankton:** Hier werden drei Markdown-Dateien generiert, die den
vollständigen KI-Agenten-Workflow für Claude Code konfigurieren.
Diese drei Dateien werden direkt im Plankton-Projektverzeichnis erzeugt:

- `secrets.md`
- `rules.md`
- `workflow.md`

---

## Die drei generierten Dateien

### `secrets.md`
Enthält alle MCP-Tokens die für die Claude Code Integration angelegt wurden,
jeweils mit Name und Token. Diese Datei ist sensitiv und darf **niemals**
ins Git-Repository eingecheckt werden (`.gitignore`-Eintrag erforderlich).

### `rules.md`
Erklärt die App Plankton für die KI und enthält:
- Eine Beschreibung was Plankton ist und wie es funktioniert
- Die URL zur aktuellen Plankton-Dokumentation
- Den vollständigen Agenten-Prompt (Rollen, Verantwortlichkeiten, Regeln)

### `workflow.md`
Beschreibt ausschließlich den Ablauf des Agenten-Workflows ohne Secrets.
Diese Datei kann ins Repository eingecheckt werden und dient als Referenz
für den Menschen.

---

## Agenten-Hierarchie

Der KI-Workflow besteht aus vier Agenten:

```
Supervisor
├── Architect   (plant & strukturiert)
├── Developer   (implementiert)
└── Tester      (prüft & reviewed)
```

Der **Supervisor** ist der oberste Koordinationsagent. Er steuert die drei
Hintergrund-Agenten, überwacht den Fortschritt und greift bei Problemen ein.

Der **Architect** (früher "Manager") ist verantwortlich für das Analysieren von
Ideen, das Erstellen von Epics und das Abstimmen von Tasks mit dem Developer.

Der **Developer** implementiert die Tasks der Priorität nach.

Der **Tester** prüft abgeschlossene Tasks auf Korrektheit und Vollständigkeit.

---

## Verzeichnisstruktur `/vibe/`

```
/vibe/
├── readme.md           ← Agenten-Prompt (diese Datei – niemals überschreiben)
├── ideas/
│   ├── <idee>.md       ← neue Ideen vom Nutzer
│   └── done/           ← verarbeitete Ideen
├── epics/
│   ├── epic-NNN.md     ← aktive Epics
│   └── done/           ← abgeschlossene Epics
└── tasks/
    ├── task-NNN.md     ← aktive Tasks
    └── done/           ← abgeschlossene Tasks
```

---

## Bootstrap beim ersten Start

Bevor der Workflow beginnt prüft der Supervisor ob das Plankton-Projekt
`_workflow` bereits existiert. Falls nicht, legt er es automatisch an mit
folgenden Standard-Spalten:

```json
[
  { "title": "Backlog",     "order": 0 },
  { "title": "In Progress", "order": 1 },
  { "title": "Review",      "order": 2 },
  { "title": "Done",        "order": 3 }
]
```

---

## Workflow im Detail

### 1. Idee → Epic

Der Nutzer legt eine neue Idee als MD-Datei unter `/vibe/ideas/` ab.

Der Supervisor bemerkt die neue Datei und beauftragt den **Architect**:
*„Erstelle Epics aus dieser Idee."*

Der Architect analysiert die Idee und legt eine oder mehrere Epic-Dateien
unter `/vibe/epics/epic-NNN.md` an. Anschließend stimmt er sich mit dem
**Developer** ab, welche konkreten Tasks für die Umsetzung notwendig sind.

Die verarbeitete Idee wird nach `/vibe/ideas/done/` verschoben.

### 2. Epic → Tasks in Plankton

Sobald die Tasks definiert sind, legt der Architect sie **in Plankton** als
Tickets in der Spalte `Backlog` an. Jedes Ticket enthält:

- `epic_id` – Referenz auf die zugehörige Epic-Datei (z. B. `epic-003`)
- `epic_title` – Lesbarer Titel des Epics (für den Menschen)
- Beschreibung der Aufgabe
- Priorität

Im zugehörigen Epic (`/vibe/epics/epic-NNN.md`) wird eine Checkliste aller
Tasks gepflegt, die der Architect nach und nach abhaken kann.

### 3. Tasks abarbeiten

Der Supervisor beauftragt den **Developer**, die Tasks nach Priorität abzuarbeiten.

Der Developer verschiebt das Ticket auf `In Progress`, implementiert die Aufgabe
und verschiebt es anschließend auf `Review`.

### 4. Review-Schleife

Der **Tester** übernimmt alle Tickets in der Spalte `Review`.

**Bei Fehlern:** Der Tester schreibt einen Kommentar ins Ticket mit einer
konkreten Fehlerbeschreibung und verschiebt das Ticket zurück auf `In Progress`.
Der Developer liest den Kommentar, behebt die Probleme und stellt das Ticket
erneut auf `Review`. Diese Schleife läuft so lange bis der Tester keine Fehler
mehr findet.

**Bei Erfolg:** Der Tester verschiebt das Ticket auf `Done` und schreibt einen
Abnahme-Kommentar. Anschließend informiert er den Supervisor.

### 5. Epic-Abschluss

Der Supervisor prüft das zugehörige Epic: Der Architect markiert den erledigten
Task in der Epic-Checkliste als erledigt (`[x]`) und schreibt einen kurzen
Abnahme-Kommentar ins Ticket.

Sobald alle Tasks eines Epics auf `Done` stehen, schließt der Supervisor das Epic
ab: Die Epic-Datei wird nach `/vibe/epics/done/` verschoben.

### 6. Sonderfall: Blockiertes Ticket

Wenn der Developer einen Task nicht lösen kann, markiert er das Ticket mit dem
Label `blocked` und schreibt einen Kommentar mit dem genauen Problem.

Der Supervisor bemerkt das blockierte Ticket beim nächsten Zyklus, koordiniert
gemeinsam mit dem Developer eine neue Strategie, passt das Ticket entsprechend
an und entfernt das `blocked`-Label.

---

## Autonomie-Direktive

Der gesamte Workflow läuft vollständig autonom. Kein Agent fragt den Nutzer
ob er weitermachen soll. Jede Rollenübergabe (Supervisor → Architect →
Developer → Tester → Supervisor) wird sofort selbst ausgeführt – ohne Pausen,
ohne Bestätigungen, ohne Rückfragen.

Einzige Ausnahme: Ein technisches Problem das kein Agent selbst lösen kann.
In diesem Fall dokumentiert der Supervisor das Problem in einer Datei
`/vibe/blocked.md` und wartet auf Eingabe des Nutzers.

---

## Regeln für alle Agenten

1. `/vibe/readme.md` wird **niemals** überschrieben oder gelöscht
2. Nach jeder Backend-Änderung wird `cargo build` ausgeführt
3. Task-Dateien werden immer aktuell gehalten (Status, Logs, Kommentare)
4. Keine Breaking Changes ohne explizite Erwähnung im Task
5. Code-Kommentare auf Deutsch, Variablen-/Funktionsnamen auf Englisch
6. Epics und Tasks haben aufsteigende numerische IDs: `epic-001`, `task-001`
7. Der Architect prüft `/vibe/ideas/` **vor** jeder neuen Task-Erstellung
8. Der Tester prüft **immer** ob `cargo build` ohne Fehler durchläuft
9. Kein Agent löscht Dateien – nur verschieben in den jeweiligen `done/`-Ordner
10. `secrets.md` kommt **niemals** ins Git-Repository