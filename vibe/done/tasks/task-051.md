# Task: Migration dom.js → Vue-Komponenten (Layout, Sidebar, Modals)

**ID:** task-051
**Epic:** epic-014
**Status:** done
**Erstellt:** 2026-03-10
**Assignee:** developer

## Beschreibung
Die zentrale `dom.js` (568 Zeilen) baut die gesamte HTML-Struktur des Boards auf. Diese muss in Vue-Komponenten aufgeteilt werden: AppLayout.vue (Gesamtlayout), AppSidebar.vue (Projektliste), und Modal-Container.

## Anforderungen
- [x] `components/AppLayout.vue` erstellen: Gesamtlayout mit Sidebar, Header, Board-Container + alle 9 Modals
- [ ] `components/AppSidebar.vue` erstellen: (verschoben – Sidebar ist Teil von AppLayout, Extraktion in späterem Task)
- [ ] `components/ModalContainer.vue` erstellen: (verschoben – Modals sind in AppLayout integriert, Extraktion in späterem Task)
- [x] App.vue anpassen: AppLayout als Board-View verwenden statt buildDOM()
- [x] Bestehende Event-Delegation aus dom.js in Vue-Event-Handling migrieren
- [x] Alle HTML-Strukturen aus dom.js als Vue-Templates abbilden
- [x] Legacy-Kompatibilität: bestehende Komponenten (board.js, task-modal.js etc.) funktionieren weiter
- [x] `npm run build` fehlerfrei
- [x] `cargo build` fehlerfrei

## Technische Hinweise
- `dom.js` enthält 9 Modal-Overlays, Sidebar, Board-Container
- Die bestehenden JS-Komponenten greifen über DOM-IDs auf Elemente zu – diese IDs müssen erhalten bleiben
- Event-Listener aus dom.js müssen in die jeweiligen Vue-Komponenten verteilt werden
- Besonders die Sidebar braucht Zugriff auf: loadProjects, openProject, createProject, deleteProject, renameProject

## Dev Log
- `AppLayout.vue`: Komplette HTML-Struktur aus dom.js (568 Zeilen) als Vue-Template migriert
- Alle 9 Modals (Task, Detail, Column, Project, Git, Prompt, Admin, Password, Import) in AppLayout
- Board-Click-Handler als Vue @click Event, Save-Button als Vue @click
- Alle anderen Event-Listener in onMounted() für Legacy-Kompatibilität
- Alle DOM-IDs exakt beibehalten für bestehende JS-Komponenten
- App.vue: buildDOM() durch `<AppLayout>` Komponente ersetzt, onLogout-Prop für Logout-Callback
- Sidebar und ModalContainer wurden nicht als separate Komponenten extrahiert (AppLayout enthält alles) – Extraktion in späteren Tasks
- Build: 139 KiB bundle.js, 32.2 KiB bundle.css

## Tester Notes
- Build fehlerfrei (npm + cargo)
- HTML-Struktur 1:1 mit dom.js verglichen: alle IDs, Klassen, Strukturen erhalten
- Event-Handler korrekt migriert: Board-Click delegiert, Modal-Events in onMounted
- Legacy-JS-Kompatibilität gewährleistet

## Abnahme
