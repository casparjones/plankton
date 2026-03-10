# Epic: Kanban-Board – vollständige Funktionalität

**ID:** epic-004
**Status:** done
**Erstellt:** 2026-03-08
**Priorität:** high

## Beschreibung
Vollständig funktionsfähiges Kanban-Board: Task-Erstellung mit sofortigem Modal, vollständiges Edit-Modal mit allen Feldern, Board-Ansicht mit Points-Badge, Worker-Avatar, Labels. Drag & Drop mit previous_row-Tracking und Log-Einträgen.

## Akzeptanzkriterien
- [x] "+ Task"-Button öffnet sofort das Edit-Modal nach Erstellung
- [x] Drag & Drop setzt previous_row und schreibt Log-Eintrag
- [x] Edit-Modal zeigt alle Felder: Titel, Beschreibung, Points, Worker, Labels, Logs (read-only), Kommentare, Erstellt/Geändert am, Vorherige Spalte (read-only), Löschen-Button
- [x] Task-Karten zeigen: Titel, Beschreibung (80 Zeichen), Points-Badge, Worker-Initial, Labels
- [x] Spalten-Header zeigt Spaltenname + Anzahl Tasks + Farbstreifen
- [x] Leere Spalten zeigen "Keine Tasks" Placeholder
- [x] _archive-Spalte wird im Board nicht angezeigt

## Tasks
- [x] task-009: Alle Unter-Tasks zusammengefasst – Modal, Karten, Header, Drag&Drop komplett

## Notizen
- Abhängig von Epic-002 (erweitertes Task-Datenmodell)
- jKanban/Dragula ist bereits integriert
