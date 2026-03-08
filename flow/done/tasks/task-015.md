# Task-015: Frontend Login-Seite & Auth-Flow

**Epic:** epic-006
**Status:** open
**Rolle:** Developer

## Beschreibung
Login-Seite im Dark Theme. App prüft beim Laden ob JWT gültig. Header zeigt eingeloggten User und Logout-Button.

## Akzeptanzkriterien
- [ ] Login-Formular: Username + Passwort + Submit-Button, zentriert, Dark Theme
- [ ] Bei erfolgreichem Login: Weiterleitung zum Board
- [ ] Bei Fehler: Fehlermeldung anzeigen
- [ ] App prüft beim Start GET /auth/me → wenn 401 → Login-Seite anzeigen
- [ ] Header/Sidebar zeigt eingeloggten display_name
- [ ] Logout-Button: POST /auth/logout → Login-Seite
- [ ] Bei must_change_password → Passwort-Ändern Dialog öffnen
