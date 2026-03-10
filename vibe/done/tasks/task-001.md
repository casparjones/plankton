# Task: tower-http TraceLayer + farbiges Request-Logging implementieren

**ID:** task-001
**Epic:** epic-001
**Status:** done
**Erstellt:** 2026-03-08
**Assignee:** developer

## Beschreibung
Implementiere eine eigene Logging-Middleware für Axum, die jeden HTTP-Request farbig im Terminal loggt. Nutze ANSI-Escape-Codes direkt (kein extra Crate nötig).

## Anforderungen
- [ ] Jeder Request wird geloggt im Format: `[YYYY-MM-DD HH:MM:SS] METHOD  /path  STATUS  DURATIONms`
- [ ] Methoden farbig: GET=grün (\x1b[32m), POST=blau (\x1b[34m), PUT=gelb (\x1b[33m), DELETE=rot (\x1b[31m)
- [ ] Status-Codes farbig: 2xx=grün, 4xx=gelb, 5xx=rot
- [ ] Dauer in Millisekunden mit 1 Dezimalstelle
- [ ] Als Axum-Layer implementieren (tower Service oder eigene Middleware-Funktion)
- [ ] Bestehende tracing_subscriber-Initialisierung beibehalten
- [ ] Kein Breaking Change an bestehenden Routen

## Technische Hinweise
- Datei: `src/main.rs`
- Implementierung als `axum::middleware::from_fn` oder als eigener Tower-Layer
- `std::time::Instant` für Zeitmessung
- ANSI Reset: `\x1b[0m`
- Middleware vor dem Router als `.layer()` einhängen
- `tower-http` features in Cargo.toml evtl. um "trace" erweitern

## Dev Log
- `src/main.rs`: Neue Middleware `request_logger` als `axum::middleware::from_fn` implementiert
- ANSI-Konstanten (RESET, GREEN, BLUE, YELLOW, RED, BOLD, DIM) als `const` definiert
- Hilfsfunktionen `method_color()` und `status_color()` für Farbzuordnung
- Middleware misst Dauer via `std::time::Instant`, loggt mit `println!` im gewünschten Format
- Middleware als `.layer(axum::middleware::from_fn(request_logger))` vor CorsLayer eingehängt
- Imports erweitert: `Request`, `Next`, `Response`, `Instant`, `Local` (chrono)
- Zusätzlich task-002 gleich mit erledigt: `print_startup_banner()` Funktion mit farbiger Routen-Tabelle
- Ungenutzten `delete`-Import aus `routing` entfernt (war Warning)
- `cargo build`: 0 errors, 0 warnings

## Tester Notes
- Code-Review: sauber, idiomatisches Rust, korrekte ANSI-Escape-Codes
- `cargo build`: 0 errors, 0 warnings
- Alle 7 Anforderungen erfüllt
- Middleware-Layer korrekt positioniert (vor CorsLayer)
- Keine Breaking Changes

## Abnahme
