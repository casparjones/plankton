# Task-017: Frontend Passwort-Ändern Dialog

**Epic:** epic-006
**Status:** open
**Rolle:** Developer

## Beschreibung
Dialog für das eigene Passwort ändern. Erfordert altes Passwort + neues Passwort (2x).

## Akzeptanzkriterien
- [ ] Modal/Overlay: altes Passwort, neues Passwort, neues Passwort bestätigen
- [ ] Validierung: neues PW muss übereinstimmen, min. 4 Zeichen
- [ ] POST /auth/change-password aufrufen
- [ ] Erfolg: Modal schließen, Bestätigung anzeigen
- [ ] Fehler: Fehlermeldung anzeigen (z.B. altes PW falsch)
- [ ] Erreichbar über User-Menü in der Sidebar
