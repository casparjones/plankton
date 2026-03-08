# Epic: Axum Middleware Logger

**ID:** epic-001
**Status:** done
**Erstellt:** 2026-03-08
**Priorität:** high

## Beschreibung
Farbiges Request-Logging im Terminal, ähnlich wie Gin in Go. Beim Serverstart sollen alle registrierten Routen tabellarisch ausgegeben werden. Jeder HTTP-Request wird mit Methode (farbig), Pfad, Status-Code (farbig) und Dauer geloggt.

## Akzeptanzkriterien
- [x] Jede HTTP-Anfrage wird geloggt mit: Methode (farbig), Pfad, Status-Code (farbig), Dauer in ms
- [x] Beim Start: alle Routen werden einmalig tabellarisch ausgegeben (Methode + Pfad)
- [x] Farben: GET=grün, POST=blau, PUT=gelb, DELETE=rot, 2xx=grün, 4xx=gelb, 5xx=rot
- [x] Startup-Banner: 🪼 Plankton v0.1.0 mit Routentabelle
- [x] Kein Breaking Change an der bestehenden API

## Tasks
- [x] task-001: tower-http TraceLayer + farbiges Request-Logging implementieren
- [x] task-002: Routen-Tabelle beim Serverstart ausgeben + Startup-Banner

## Notizen
- Abhängigkeiten: `tower-http` (TraceLayer, features: trace), `tracing`, `tracing-subscriber` mit `EnvFilter`
- `tower-http` ist bereits in Cargo.toml, braucht aber evtl. Feature `trace`
- Farben via ANSI-Escape-Codes oder `colored` Crate
