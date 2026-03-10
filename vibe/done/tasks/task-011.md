# Task-011: Auth-Endpunkte (login, logout, me, change-password)

**Epic:** epic-006
**Status:** open
**Rolle:** Developer

## Beschreibung
Auth-Endpunkte implementieren: Login gibt JWT als HttpOnly-Cookie zurück, Logout löscht Cookie, /me gibt eigene Info, change-password erfordert altes Passwort.

## Akzeptanzkriterien
- [ ] POST /auth/login: username+password → JWT HttpOnly Cookie (8h Laufzeit)
- [ ] POST /auth/logout: Cookie löschen (Set-Cookie mit Max-Age=0)
- [ ] GET /auth/me: eigene User-Info (ohne password_hash)
- [ ] POST /auth/change-password: old_password + new_password, verifiziert altes PW
- [ ] JWT enthält: user_id, username, display_name, role, exp
- [ ] JWT_SECRET aus Env-Variable oder zufällig generiert beim Start
