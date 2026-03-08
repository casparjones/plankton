# Task-014: Bootstrap-Logik (Standard-Admin beim ersten Start)

**Epic:** epic-006
**Status:** open
**Rolle:** Developer

## Beschreibung
Beim ersten Start prüft Plankton ob ein Admin existiert. Wenn nicht, wird ein Standard-Admin (admin/admin) angelegt. Beim Login mit Standard-PW → Hinweis Passwort ändern.

## Akzeptanzkriterien
- [ ] Beim Start: prüfe ob mindestens ein User mit role=admin existiert
- [ ] Wenn kein Admin: lege User an (username: admin, password: admin, role: admin)
- [ ] Login-Response enthält Flag "must_change_password" wenn PW == "admin"
- [ ] Startup-Log zeigt ob Default-Admin angelegt wurde
