# Task: Models extrahieren (project.rs, requests.rs)

**ID:** task-028
**Epic:** epic-009
**Status:** done
**Erstellt:** 2026-03-08
**Assignee:** developer

## Beschreibung
Alle Datenmodell-Structs aus main.rs in src/models/ extrahieren.

## Anforderungen
- [ ] src/models/mod.rs erstellen
- [ ] src/models/project.rs: ProjectDoc, Column, User, Task, Default for Task
- [ ] src/models/auth.rs: AuthUser, Claims, LoginRequest, ChangePasswordRequest, CreateAuthUserRequest, UpdateAuthUserRequest, ResetPasswordRequest, AgentToken, CreateTokenRequest, UpdateTokenRequest, default_true(), generate_agent_token()
- [ ] src/models/requests.rs: DeleteQuery, GetProjectQuery, MoveTaskRequest, McpCall, ToolDef, JsonRpcRequest, JsonRpcResponse, JsonRpcError
- [ ] Alle Structs als `pub` markieren, alle Felder als `pub`
- [ ] In main.rs: `mod models; use models::*;` und die alten Definitionen entfernen
- [ ] `cargo build` erfolgreich

## Technische Hinweise
- Betroffene Zeilen in main.rs: ca. 75-335
- Serde-Derives und Attribute müssen mit
- Default-Impl für Task muss mit
