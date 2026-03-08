# Task: Routen-Tabelle beim Serverstart ausgeben + Startup-Banner

**ID:** task-002
**Epic:** epic-001
**Status:** done
**Erstellt:** 2026-03-08
**Assignee:** developer

## Beschreibung
Beim Serverstart soll ein Startup-Banner mit Plankton-Logo und eine tabellarische Auflistung aller registrierten Routen im Terminal ausgegeben werden.

## Anforderungen
- [ ] Startup-Banner: `🪼 Plankton v0.1.0` mit Trennlinie (━━━)
- [ ] Alle API-Routen tabellarisch ausgeben: Methode (farbig) + Pfad
- [ ] Routen nach Pfad sortiert
- [ ] Abschließende Trennlinie + `listening on 0.0.0.0:PORT`
- [ ] Methoden farbig wie in task-001: GET=grün, POST=blau, PUT=gelb, DELETE=rot

## Technische Hinweise
- Datei: `src/main.rs` in der `main()`-Funktion
- Routen als statisches Array definieren (Axum bietet keinen Router-Introspection)
- `println!` mit ANSI-Farben verwenden
- Ausgabe vor `axum::serve()` aufrufen

## Dev Log
- `print_startup_banner(&port)` Funktion implementiert in `src/main.rs`
- 18 Routen als statisches Array `&[(&str, &str)]` definiert
- Banner: 🪼 Plankton v0.1.0 + Trennlinien (━━━) + farbige Methoden + listening-Zeile
- Aufruf in `main()` vor `axum::serve()`
- Routen sind nach logischer Gruppierung sortiert (projects → tasks → columns → users → events → mcp)
- Mit task-001 zusammen implementiert, selber Build

## Tester Notes
- Code-Review: Routen-Array vollständig (18 Routen, alle aus dem Router)
- Banner-Format entspricht exakt der Spezifikation aus der readme
- Farbzuordnung korrekt, BOLD für Methoden
- `cargo build`: 0 errors, 0 warnings
- Alle 5 Anforderungen erfüllt

## Abnahme
