// Markdown-Generator für die drei KI-Agenten-Konfigurationsdateien.
// Erzeugt secrets.md, rules.md und workflow.md als Strings.

export interface TokenEntry {
  name: string
  token: string
  role: string
}

/**
 * Erzeugt secrets.md mit allen MCP-Token-Paaren.
 */
export function generateSecretsMd(tokens: TokenEntry[], planktonUrl: string): string {
  const tokenLines = tokens.map(t =>
    `| ${t.name} | ${t.role} | \`${t.token}\` |`
  ).join('\n')

  return `# Plankton Secrets

> **ACHTUNG:** Diese Datei enthält sensible Tokens und darf **niemals** ins Git-Repository eingecheckt werden.
> Füge \`secrets.md\` zur \`.gitignore\` hinzu!

## Plankton-Server

- **URL:** ${planktonUrl}

## MCP Agent-Tokens

| Name | Rolle | Token |
|------|-------|-------|
${tokenLines}

## Verwendung

Tokens werden als Bearer-Token im Authorization-Header verwendet:

\`\`\`
Authorization: Bearer plk_xxxxx...
\`\`\`

### Token-Verwaltung

Tokens können in der Plankton-Oberfläche unter **Admin (Zahnrad-Icon) → Tokens** verwaltet werden.
Dort können neue Tokens erstellt, bestehende deaktiviert oder gelöscht werden.
`
}

/**
 * Erzeugt rules.md mit App-Beschreibung und Agenten-Prompt.
 */
export function generateRulesMd(planktonUrl: string, projectName: string): string {
  return `# Plankton – KI-Agenten Regeln

## Was ist Plankton?

Plankton ist ein Kanban-Board-System mit REST-API und MCP-Integration.
Es ermöglicht KI-Agenten, Tasks über das Model Context Protocol (MCP) zu verwalten.

- **Server:** ${planktonUrl}
- **Projekt:** ${projectName}
- **Dokumentation:** ${planktonUrl}/docs

## MCP-Tools

Folgende MCP-Tools stehen zur Verfügung (je nach Rolle):

### Öffentlich
- \`list_projects\` – Alle Projekte auflisten
- \`get_project\` – Ein Projekt laden
- \`summarize_board\` – Board-Übersicht (Spalten + Anzahl Tasks)

### Manager / Architect
- \`create_project\` – Neues Projekt anlegen
- \`list_epics\` – Spalten als Epics anzeigen
- \`create_task\` – Neuen Task erstellen
- \`move_task\` – Task in andere Spalte verschieben
- \`assign_task\` – Worker zuweisen
- \`delete_task\` – Task löschen

### Developer
- \`update_task\` – Task bearbeiten
- \`get_assigned_tasks\` – Eigene zugewiesene Tasks
- \`add_log\` – Log-Eintrag hinzufügen
- \`submit_for_review\` – Task zur Review einreichen

### Tester
- \`get_review_queue\` – Tasks in Review
- \`add_comment\` – Kommentar hinzufügen
- \`approve_task\` – Task abnehmen (→ Done)
- \`reject_task\` – Task zurückweisen (→ In Progress)

## Agenten-Hierarchie

\`\`\`
Supervisor (steuert den gesamten Workflow)
├── Architect   (plant, erstellt Epics und Tasks)
├── Developer   (implementiert Tasks)
└── Tester      (prüft und reviewed Tasks)
\`\`\`

## Regeln

1. Jeder Agent arbeitet nur mit seinem MCP-Token und den damit verfügbaren Tools
2. Kommunikation erfolgt über Task-Kommentare und -Logs in Plankton
3. Der Workflow läuft vollständig autonom ohne Rückfragen an den Nutzer
4. Code-Kommentare auf Deutsch, Variablen-/Funktionsnamen auf Englisch
5. Nach jeder Backend-Änderung wird \`cargo build\` ausgeführt
6. Keine Breaking Changes ohne explizite Erwähnung im Task
7. Bei Blockaden: Label \`blocked\` setzen und Kommentar mit Problembeschreibung
`
}

/**
 * Erzeugt workflow.md mit dem vollständigen Agenten-Workflow.
 */
export function generateWorkflowMd(): string {
  return `# Plankton – KI-Agenten Workflow

> Diese Datei beschreibt den autonomen Workflow der KI-Agenten.
> Sie enthält keine Secrets und kann ins Repository eingecheckt werden.

## Übersicht

Der Workflow besteht aus vier Agenten, die vollständig autonom arbeiten:

| Agent | Rolle | Verantwortung |
|-------|-------|---------------|
| **Supervisor** | Koordination | Steuert alle Agenten, überwacht Fortschritt |
| **Architect** | Planung | Analysiert Ideen, erstellt Epics und Tasks |
| **Developer** | Umsetzung | Implementiert Tasks nach Priorität |
| **Tester** | Qualität | Prüft Code auf Korrektheit und Vollständigkeit |

## Ablauf

### 1. Idee → Epic

Der Nutzer beschreibt eine Idee. Der **Architect** analysiert sie und erstellt
ein oder mehrere Epics mit konkreten Akzeptanzkriterien.

### 2. Epic → Tasks

Der Architect bricht Epics in konkrete Tasks auf und legt sie im Kanban-Board
in der Spalte \`Backlog\` an. Jeder Task enthält:
- Referenz auf das zugehörige Epic
- Detaillierte Beschreibung und Anforderungen
- Priorität und geschätzte Story Points

### 3. Entwicklung

Der **Developer** nimmt Tasks nach Priorität aus dem Backlog:
1. Task auf \`In Progress\` verschieben
2. Code implementieren
3. Log-Eintrag mit Änderungen schreiben
4. Task auf \`Review\` verschieben (\`submit_for_review\`)

### 4. Review-Schleife

Der **Tester** prüft alle Tasks in \`Review\`:

**Bei Fehlern:**
- Kommentar mit konkreter Fehlerbeschreibung
- Task zurück auf \`In Progress\` (\`reject_task\`)
- Developer behebt und reicht erneut ein

**Bei Erfolg:**
- Abnahme-Kommentar schreiben
- Task auf \`Done\` verschieben (\`approve_task\`)

### 5. Epic-Abschluss

Der **Supervisor** prüft regelmäßig den Fortschritt:
- Erledigte Tasks im Epic abhaken
- Wenn alle Tasks \`Done\`: Epic als abgeschlossen markieren
- Nächstes Epic starten

### 6. Blockaden

Wenn ein Developer einen Task nicht lösen kann:
1. Label \`blocked\` setzen
2. Kommentar mit genauem Problem
3. Supervisor koordiniert neue Strategie

## Autonomie-Direktive

Der gesamte Workflow läuft **vollständig autonom**. Kein Agent fragt den Nutzer
ob er weitermachen soll. Jede Rollenübergabe wird sofort ausgeführt – ohne
Pausen, ohne Bestätigungen, ohne Rückfragen.

Einzige Ausnahme: Ein technisches Problem das kein Agent selbst lösen kann.
`
}
