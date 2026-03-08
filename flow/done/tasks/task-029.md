# Task: Store extrahieren (mod.rs, couch.rs, file.rs)

**ID:** task-029
**Epic:** epic-009
**Status:** done
**Erstellt:** 2026-03-08
**Assignee:** developer

## Beschreibung
DataStore enum, CouchDb struct und FileStore struct aus main.rs in src/store/ extrahieren.

## Anforderungen
- [ ] src/store/mod.rs: DataStore enum + Delegation
- [ ] src/store/couch.rs: CouchDb struct + impl
- [ ] src/store/file.rs: FileStore struct + impl
- [ ] In main.rs: `mod store; use store::*;`
- [ ] `cargo build` erfolgreich

## Technische Hinweise
- DataStore, CouchDb, FileStore Structs sind in Zeilen 50-73
- CouchDb impl ab Zeile ~2164
- FileStore impl ab Zeile ~2447
- DataStore impl ab Zeile ~2261
- Store-Methoden nutzen ApiError → error.rs muss entweder inline bleiben oder gleichzeitig extrahiert werden
