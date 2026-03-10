# Idee: Letztes Projekt im localStorage merken

## Kernidee

Beim Wechsel des aktiven Projekts wird die Projekt-ID im localStorage
des Browsers gespeichert. Beim nächsten Öffnen der App (oder nach
einem Login) wird automatisch das zuletzt gewählte Projekt geladen
statt immer das erste Projekt in der Liste.

## Verhalten

- Nutzer wählt Projekt "Backend Refactoring" → wird im localStorage gespeichert
- Nutzer schließt den Browser oder loggt sich aus
- Beim nächsten Login: App lädt automatisch "Backend Refactoring" statt
  dem ersten Projekt in der Liste
- Existiert das gespeicherte Projekt nicht mehr (gelöscht):
  → Fallback auf das erste Projekt in der Liste
  → Alten localStorage-Eintrag bereinigen

## Technische Umsetzung

**localStorage Key:** `plankton_last_project_id`

**Wann speichern:**
- Immer wenn der Nutzer ein Projekt in der Sidebar anklickt
- Immer wenn ein neues Projekt erstellt wird (das neue Projekt wird aktiv)

**Beim App-Start (nach Login):**
1. `localStorage.getItem('plankton_last_project_id')` lesen
2. Projekte vom Backend laden
3. Gibt es ein Projekt mit dieser ID in der Liste?
   → Ja: dieses Projekt öffnen
   → Nein: erstes Projekt öffnen, localStorage-Eintrag löschen

**Pro Nutzer:**
Wenn später ein Login-System existiert, soll der Key den Nutzernamen
enthalten damit verschiedene Nutzer am gleichen Browser ihr eigenes
letztes Projekt behalten:
`plankton_last_project_id_<username>`

## Betroffene Dateien

- `src/frontend/services/project-service.js` (nach Refactoring)
- oder aktuell: `static/main.js` in den Funktionen openProject() und createProject()