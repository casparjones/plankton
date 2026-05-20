# REST-API-Referenz

Alle JSON-Responses folgen dem Format `{"error": "...", "code": "OPTIONAL_CODE"}` bei Fehlern.

## Auth (öffentlich, kein Guard)

| Methode | Route | Beschreibung |
|---------|-------|-------------|
| POST | `/auth/login` | Username + Passwort → JWT-Cookie (`plankton_token`) |
| POST | `/auth/logout` | Löscht JWT-Cookie |
| GET | `/auth/me` | Aktuellen User abrufen (aus JWT-Claims) |
| POST | `/auth/change-password` | Passwort ändern → neuer JWT |

## OAuth 2.0 (öffentlich)

| Methode | Route | Beschreibung |
|---------|-------|-------------|
| GET | `/authorize` | Authorization Endpoint (Consent-Screen) |
| POST | `/token` | Token Endpoint: `authorization_code` oder `refresh_token` |
| POST | `/register` | Dynamic Client Registration (RFC 7591) |
| GET | `/.well-known/oauth-authorization-server` | Server Metadata (RFC 8414) |
| GET | `/.well-known/oauth-protected-resource` | Protected Resource Metadata |

Aliases: `/oauth/authorize`, `/oauth/token`, `/oauth/register` funktionieren identisch.

## CLI Device Flow (öffentlich)

| Methode | Route | Beschreibung |
|---------|-------|-------------|
| POST | `/auth/cli-init` | Startet Login-Session → `{session_id, code, login_url}` |
| GET | `/auth/cli-poll/:session_id` | Pollt Status → `{status: "pending"|"approved"|"expired", token?}` |
| POST | `/auth/cli-approve` | Genehmigt Session (mit gültigem JWT) |
| GET | `/cli-login` | Login-Seite für Browser |
| GET | `/install` | Bash-Installationsskript für CLI |

## Projekte (Auth erforderlich)

| Methode | Route | Beschreibung |
|---------|-------|-------------|
| GET | `/api/projects` | Alle Projekte auflisten |
| POST | `/api/projects` | Neues Projekt erstellen (4 Standard-Spalten) |
| GET | `/api/projects/:id` | Ein Projekt (UUID oder Slug) |
| PUT | `/api/projects/:id` | Projekt aktualisieren (Rev-Check erforderlich) |
| DELETE | `/api/projects/:id` | Projekt löschen |

## Tasks (Auth erforderlich)

| Methode | Route | Beschreibung |
|---------|-------|-------------|
| POST | `/api/projects/:id/tasks` | Neue Task erstellen (landet in erster Spalte) |
| PUT | `/api/projects/:id/tasks/:task_id` | Task partiell aktualisieren |
| DELETE | `/api/projects/:id/tasks/:task_id` | Task löschen |
| POST | `/api/projects/:id/tasks/:task_id/move` | Task in andere Spalte verschieben |
| POST | `/api/projects/:id/tasks/reorder` | Tasks in Spalte umsortieren |
| POST | `/api/projects/:id/tasks/batch-move` | Mehrere Tasks auf einmal verschieben |
| POST | `/api/projects/:id/import` | Bulk-Import (CSV/JSON) |

## Spalten (Auth erforderlich)

| Methode | Route | Beschreibung |
|---------|-------|-------------|
| POST | `/api/projects/:id/columns` | Neue Spalte |
| PUT | `/api/projects/:id/columns/:column_id` | Spalte aktualisieren |
| DELETE | `/api/projects/:id/columns/:column_id` | Spalte löschen (nur wenn nicht `locked`) |

## Team-User (Auth erforderlich)

| Methode | Route | Beschreibung |
|---------|-------|-------------|
| POST | `/api/projects/:id/users` | Team-Mitglied hinzufügen |
| PUT | `/api/projects/:id/users/:user_id` | Team-Mitglied aktualisieren |
| DELETE | `/api/projects/:id/users/:user_id` | Team-Mitglied entfernen |
| GET | `/api/users` | Alle System-User auflisten (öffentlich) |

## Echtzeit (Auth erforderlich)

| Methode | Route | Beschreibung |
|---------|-------|-------------|
| GET | `/api/projects/:id/events` | SSE-Stream für Projekt-Events |

Siehe [realtime-sse.md](realtime-sse.md) für Event-Formate.

## Admin (Auth + role=admin)

| Methode | Route | Beschreibung |
|---------|-------|-------------|
| GET | `/api/admin/users` | Alle System-User |
| POST | `/api/admin/users` | Neuen User anlegen |
| PUT | `/api/admin/users/:user_id` | User aktualisieren |
| DELETE | `/api/admin/users/:user_id` | User löschen |
| PUT | `/api/admin/users/:user_id/password` | Passwort zurücksetzen |
| GET | `/api/admin/tokens` | Alle Agent-Tokens |
| POST | `/api/admin/tokens` | Neuen Token erstellen |
| PUT | `/api/admin/tokens/:token_id` | Token aktualisieren |
| DELETE | `/api/admin/tokens/:token_id` | Token löschen |
| GET | `/api/admin/oauth-clients` | Alle OAuth-Clients |
| POST | `/api/admin/oauth-clients` | Neuen Client anlegen |

## MCP (Auth erforderlich)

| Methode | Route | Beschreibung |
|---------|-------|-------------|
| POST | `/mcp` | JSON-RPC 2.0 Request |
| GET | `/mcp` | Streamable HTTP (SSE, MCP-Transport) |
| DELETE | `/mcp` | MCP-Session beenden |
| GET | `/mcp/tools` | Verfügbare Tools auflisten (Legacy) |

Siehe [mcp-server.md](mcp-server.md) für Tool-Referenz und Transport-Details.

## Sonstiges (öffentlich)

| Methode | Route | Beschreibung |
|---------|-------|-------------|
| GET | `/healthz` | `{"status":"ok"}` |
| GET | `/docs` | Dokumentations-Seite |
| GET | `/skill.md` | MCP-Skill-Beschreibung für `plankton skill install` |
| GET | `/p/*` | SPA-Fallback → index.html |
| GET | `/import` | SPA-Fallback → index.html |
