# Idee: Plankton als MCP-Server für LLM-Agenten

## Kernidee

Plankton soll als vollwertiger MCP-Server (Model Context Protocol) fungieren,
sodass LLM-Agenten (Claude Code, Claude Desktop, Cursor, etc.) direkt mit
Plankton arbeiten können – ohne Dateisystem-Umwege wie hier z.B. über /flow/.

Der bisherige /flow/-Workflow (Manager → Entwickler → Tester als Dateien)
soll langfristig in Plankton selbst abgebildet werden. Plankton wird damit
das Werkzeug, mit dem KI-Agenten ihre eigene Arbeit organisieren.

## Token-System

- Tokens können im Frontend generiert werden (Name + Rolle)
- Jeder Token repräsentiert einen Agenten: z.B. "Claude Manager", "Claude Developer", "Claude Tester"
- Der Token wird bei jedem API-Call mitgeschickt (Authorization: Bearer <token>)
- Plankton resolved daraus automatisch creator/worker in Tasks und Logs
- Beispiel Log-Einträge:
    - "2025-03-08 14:32 [Claude Manager] Task erstellt"
    - "2025-03-08 16:12 [Claude Tester] Review-Kommentar hinzugefügt"
- Tokens können deaktiviert/widerrufen werden

## MCP-Protokoll

- Echtes MCP (JSON-RPC über HTTP/SSE) in main.rs implementieren
- Der bestehende /mcp/-Endpunkt wird auf das offizielle MCP-Protokoll umgestellt
- Jede Rolle bekommt nur die Tools zu sehen die sie braucht (role-based visibility):
    - Manager-Tools: list_epics, create_epic, create_task, assign_task, close_epic
    - Developer-Tools: get_assigned_tasks, update_task, add_log, submit_for_review
    - Tester-Tools: get_review_queue, add_comment, approve_task, reject_task

## Dokumentations-Seite

- Eine statische HTML-Seite (/docs) erklärt LLM-Agenten wie Plankton funktioniert
- Die Seite ist maschinenlesbar optimiert (klare Struktur, keine unnötigen Grafiken)
- Inhalt: API-Referenz, Tool-Liste, Workflow-Beschreibung, Token-Setup
- Diese Seite kann direkt als System-Prompt-Quelle für Agenten dienen

## Ziel

Statt Dateien in /flow/ zu schreiben verbindet sich Claude Code direkt zu
Plankton per MCP, holt seinen nächsten Task, arbeitet ihn ab, loggt alles
in Plankton und übergibt an den nächsten Agenten – alles innerhalb von Plankton,
sichtbar im Board, nachvollziehbar über die Logs.