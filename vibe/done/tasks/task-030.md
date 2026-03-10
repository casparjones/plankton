# Task: State und Config als eigene Module extrahieren

**ID:** task-030
**Epic:** epic-009
**Status:** done
**Erstellt:** 2026-03-08
**Assignee:** developer

## Beschreibung
AppState struct in src/state.rs extrahieren. Error ist bereits in src/error.rs.
Config-Logik (Env-Variablen) in src/config.rs extrahieren.

## Anforderungen
- [ ] src/state.rs: AppState struct
- [ ] src/config.rs: Config struct mit from_env() Methode
- [ ] In main.rs: `mod state; mod config; use state::*; use config::*;`
- [ ] `cargo build` erfolgreich

## Technische Hinweise
- AppState enthält store, events und jwt_secret
- Config: COUCHDB_URI, COUCHDB_DB, PORT, JWT_SECRET
