# Plankton mit claude.ai verbinden — Setup-Guide

Dieser Guide erklärt Schritt für Schritt, wie du Plankton als MCP-Connector in **claude.ai** einbindest. Danach kann Claude direkt auf dein Kanban-Board zugreifen: Tasks erstellen, Board abfragen, Review-Queue verwalten und vieles mehr.

---

## Voraussetzungen

- Ein Plankton-Konto (lokal oder auf `https://plankton.tiny-dev.de`)
- Zugang zu [claude.ai](https://claude.ai) (Pro- oder Team-Plan)

---

## 1. Connector in claude.ai einrichten (Endnutzer)

### Schritt 1 — Connector-Einstellungen öffnen

1. claude.ai aufrufen und einloggen
2. Oben rechts auf dein Profilbild → **Settings**
3. Im linken Menü: **Connectors** (oder **Integrations**)
4. Klick auf **Add custom connector**

### Schritt 2 — MCP-URL eingeben

Im Feld **Connector URL** eingeben:

```
https://plankton.tiny-dev.de/mcp
```

Für eine selbst gehostete Instanz entsprechend die eigene URL verwenden, z.B. `https://mein-plankton.example.com/mcp`.

### Schritt 3 — OAuth-Autorisierung

Nach dem Speichern der URL öffnet sich automatisch ein **OAuth-Popup**:

1. Im Popup erscheint der Plankton-Login-Screen
2. Mit deinen Plankton-Zugangsdaten einloggen
3. Den Consent-Screen lesen — er zeigt, welche Berechtigungen claude.ai erhält
4. Auf **Zugriff erlauben** klicken
5. Das Popup schließt sich automatisch

### Schritt 4 — Verbindung testen

Zurück in claude.ai sollte der Connector jetzt als **verbunden** angezeigt werden. In einem neuen Chat kannst du sofort loslegen:

```
Zeig mir alle meine Plankton-Projekte.
```

Claude listet daraufhin alle Projekte mit ID, Titel und Task-Anzahl auf.

---

## 2. OAuth-App anlegen (für Entwickler / Self-Hosting)

claude.ai registriert sich automatisch als OAuth-Client via **Dynamic Client Registration** (RFC 7591) — du musst nichts manuell konfigurieren.

Falls du eine eigene App manuell registrieren möchtest (z.B. für Tests oder einen eigenen MCP-Client):

```bash
curl -s -X POST https://plankton.tiny-dev.de/register \
  -H "Content-Type: application/json" \
  -d '{
    "client_name": "Meine App",
    "redirect_uris": ["https://meine-app.example.com/callback"],
    "token_endpoint_auth_method": "none"
  }'
```

**Antwort:**
```json
{
  "client_id": "abc123",
  "client_name": "Meine App",
  "redirect_uris": ["https://meine-app.example.com/callback"]
}
```

### Discovery-Endpoints

claude.ai findet die OAuth-Konfiguration automatisch über:

| Endpoint | RFC | Inhalt |
|----------|-----|--------|
| `GET /.well-known/oauth-authorization-server` | RFC 8414 | Authorization Server Metadata |
| `GET /.well-known/oauth-protected-resource` | RFC 9728 | Protected Resource Metadata |

```bash
# Metadata abrufen
curl https://plankton.tiny-dev.de/.well-known/oauth-authorization-server
```

### Authorization Code Flow mit PKCE

```
1. GET /oauth/authorize?client_id=...&redirect_uri=...&response_type=code
                       &state=...&code_challenge=...&code_challenge_method=S256

2. User loggt ein + erlaubt Zugriff

3. Redirect: https://deine-app/callback?code=AUTH_CODE&state=...

4. POST /oauth/token
   { "grant_type": "authorization_code",
     "code": "AUTH_CODE",
     "code_verifier": "...",
     "client_id": "..." }

5. Response: { "access_token": "...", "refresh_token": "...", "token_type": "Bearer" }
```

**Wichtig:** Nur `code_challenge_method=S256` wird unterstützt (kein `plain`).

### Token erneuern (Refresh)

```bash
curl -s -X POST https://plankton.tiny-dev.de/oauth/token \
  -H "Content-Type: application/json" \
  -d '{
    "grant_type": "refresh_token",
    "refresh_token": "<dein-refresh-token>",
    "client_id": "<client_id>"
  }'
```

---

## 3. MCP-Tool-Übersicht

Nach der Verbindung stehen Claude folgende Tools zur Verfügung:

### Projekt-Tools

| Tool | Beschreibung | Typische Verwendung |
|------|-------------|---------------------|
| `list_projects` | Alle Projekte auflisten | "Welche Projekte gibt es?" |
| `get_project` | Projekt mit Spalten und Tasks laden | "Zeig mir das Board von Projekt X" |
| `create_project` | Neues Projekt anlegen | "Erstell ein neues Projekt 'Website Relaunch'" |
| `update_project` | Projektmetadaten ändern | "Benenne Projekt X in Y um" |
| `summarize_board` | Kompakte Board-Übersicht | "Wie viele Tasks sind gerade in Progress?" |
| `list_epics` | Epics/Spalten auflisten | "Liste alle Epics auf" |

### Task-Tools

| Tool | Beschreibung | Typische Verwendung |
|------|-------------|---------------------|
| `get_task` | Task-Details mit Kommentaren und Logs | "Zeig mir Task #abc mit allen Kommentaren" |
| `create_task` | Neue Task erstellen | "Erstell eine Task 'Login-Bug fixen'" |
| `update_task` | Task bearbeiten | "Setze Story Points auf 5" |
| `move_task` | Task in andere Spalte verschieben | "Verschiebt Task X nach In Progress" |
| `delete_task` | Task löschen | "Lösch diese doppelte Task" |
| `assign_task` | Bearbeiter zuweisen | "Weise Task X dem User frank zu" |
| `get_assigned_tasks` | Zugewiesene Tasks abrufen | "Was sind meine offenen Tasks?" |
| `reorder_tasks` | Reihenfolge in Spalte ändern | "Sortiere diese Tasks nach Priorität" |

### Review/Workflow-Tools

| Tool | Beschreibung | Typische Verwendung |
|------|-------------|---------------------|
| `submit_for_review` | Task zur Review einreichen | "Reiche Task X zur Review ein" |
| `approve_task` | Task genehmigen → Done | "Genehmige Task X" |
| `reject_task` | Task zurückweisen → In Progress | "Weise Task X ab, Grund: Tests fehlen" |
| `get_review_queue` | Tasks in Testing anzeigen | "Was liegt zur Review an?" |

### Kommunikations- und Relations-Tools

| Tool | Beschreibung |
|------|-------------|
| `add_comment` | Kommentar zu einer Task hinzufügen |
| `add_log` | Log-Eintrag schreiben (technisches Journal) |
| `add_relation` | Beziehung erstellen: `blocks` oder `subtask` |
| `remove_relation` | Beziehung entfernen |
| `list_subtasks` | Subtasks eines Epics auflisten |

### Rollenbasierter Zugriff

Die erlaubten Tools hängen von der Rolle des eingeloggten Users ab:

| Rolle | Zugriff |
|-------|---------|
| `admin`, `user` | Alle Tools |
| `manager` | Alle Tools außer `delete_task` |
| `developer` | Lesen, Task bearbeiten, kommentieren, zur Review einreichen |
| `tester` | Lesen, Review-Queue, Tasks genehmigen/ablehnen, kommentieren |

---

## 4. Beispiel-Prompts für typische Workflows

### Board abfragen

```
Gib mir eine Übersicht des Plankton-Boards. Wie viele Tasks sind in welcher Spalte?
```

```
Zeig mir alle Tasks im Plankton-Projekt, die gerade "In Progress" sind.
```

```
Was sind meine aktuell zugewiesenen Tasks?
```

### Task erstellen

```
Erstell im Plankton-Projekt eine neue Task mit dem Titel "OAuth Token Refresh fixen"
und der Beschreibung "Nach 8 Stunden schlägt der Token-Refresh fehl. Reproduzierbar
mit curl. Story Points: 5."
```

```
Ich brauche drei Tasks für das Login-Feature: Formular erstellen,
Validierung implementieren, Tests schreiben. Erstell sie alle im Plankton-Projekt.
```

### Review-Queue verwalten

```
Zeig mir die aktuelle Review-Queue im Plankton-Projekt.
Sind alle Tasks vollständig beschrieben?
```

```
Genehmige Task f4ff048e — die Dokumentation ist vollständig und korrekt.
```

```
Weise Task abc123 zurück. Begründung: Unit-Tests fehlen für Edge Case "leerer String".
```

### Workflow-Status verfolgen

```
Welche Tasks haben das Label "bug" und sind noch nicht in Done?
```

```
Füge zur Task xyz einen Kommentar hinzu: "Deployment auf Staging erfolgreich,
bereit für Production-Deploy."
```

```
Erstell eine Subtask-Beziehung: Task B ist Subtask von Epic A.
```

---

## 5. Fehlerbehebung (OAuth-Probleme)

### "OAuth popup schließt sich sofort ohne Login"

**Ursache:** Browser blockiert Popups von claude.ai.

**Lösung:**
1. In den Browser-Einstellungen Popups für `claude.ai` erlauben
2. Alternativ: Adblocker/uBlock Origin deaktivieren während des OAuth-Flows
3. Im Browser: Einstellungen → Datenschutz → Popups und Weiterleitungen → `claude.ai` zur Ausnahmeliste hinzufügen

---

### "Connector zeigt 'Verbindungsfehler' oder 'Unauthorized'"

**Ursachen und Lösungen:**

| Symptom | Mögliche Ursache | Lösung |
|---------|-----------------|--------|
| `401 Unauthorized` | Token abgelaufen | Connector trennen und neu verbinden (OAuth erneut durchlaufen) |
| `403 Forbidden` | Falsche Rolle | Im Plankton-Admin-Panel Rolle des Users prüfen |
| `Connection refused` | Server nicht erreichbar | Plankton-Instanz-URL prüfen, Server-Status kontrollieren |
| Timeout | Netzwerkproblem oder Server überlastet | Erneut versuchen, Server-Logs prüfen |

---

### "Der Consent-Screen erscheint nicht / redirect funktioniert nicht"

**Ursache:** Falsche Redirect-URI oder Client nicht registriert.

**Prüfung:**
```bash
# Discovery abrufen und Authorization Endpoint prüfen
curl -s https://plankton.tiny-dev.de/.well-known/oauth-authorization-server | jq .
```

**Für self-hosted Instanzen:** Sicherstellen, dass `PLANKTON_BASE_URL` korrekt gesetzt ist (wird in OAuth-Redirects verwendet).

---

### "Token wird nicht akzeptiert nach erneutem Login"

**Ursache:** Refresh Token Rotation — nach jedem Refresh wird ein neuer Refresh Token ausgestellt, der alte ist ungültig.

**Lösung:** Die App muss den neuen Refresh Token persistieren. Bei claude.ai passiert das automatisch.

---

### "Tool X steht nicht zur Verfügung"

**Ursache:** Die Rolle des eingeloggten Users hat keinen Zugriff auf dieses Tool.

**Beispiel:** Ein User mit Rolle `tester` kann keine Tasks erstellen (`create_task` ist nicht erlaubt).

**Lösung:** Im Plankton-Admin-Panel (`/admin`) die Rolle des Users auf `developer`, `manager` oder `admin` setzen.

---

### Verbindung zurücksetzen

Falls nichts hilft: Connector in claude.ai entfernen und neu einrichten:

1. claude.ai → Settings → Connectors
2. Plankton-Connector → **Entfernen / Disconnect**
3. In Plankton: Admin-Panel → Tokens → OAuth-Clients — den claude.ai-Client löschen (optional, aber empfohlen)
4. Connector neu einrichten (ab Schritt 1 dieser Anleitung)

---

## Weiterführende Dokumentation

- [MCP-Server Referenz](mcp-server.md) — Alle Tools, JSON-RPC-Transport, Rollen-Zugriff
- [OAuth 2.0 Auth-Flow](auth-oauth.md) — Technische Details zu JWT, PKCE, Token-Rotation
- [API-Referenz](api-reference.md) — REST-Endpunkte für direkte API-Nutzung
- [Architektur-Übersicht](architecture-overview.md) — Gesamtarchitektur des Systems
