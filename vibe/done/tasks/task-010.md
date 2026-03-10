# Task-010: Nutzer-Datenmodell & Store-Implementierung

**Epic:** epic-006
**Status:** open
**Rolle:** Developer

## Beschreibung
AuthUser-Struct in main.rs definieren und UserStore-Implementierung für FileStore und CouchDB. Nutzer werden in data/users/ als JSON gespeichert.

## Akzeptanzkriterien
- [ ] AuthUser struct: id, username, display_name, password_hash, role, created_at, updated_at, active
- [ ] UserStore trait/Methoden: list_users, get_user, get_user_by_username, create_user, update_user, delete_user
- [ ] FileStore: data/users/<id>.json
- [ ] CouchDB: users-Collection oder type-Feld
- [ ] Argon2id Passwort-Hashing (argon2 crate)
- [ ] Cargo.toml: argon2, jsonwebtoken Abhängigkeiten
