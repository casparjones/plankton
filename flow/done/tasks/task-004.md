# Task: Archiv-Logik mit Background-Task implementieren

**ID:** task-004
**Epic:** epic-002
**Status:** done
**Erstellt:** 2026-03-08
**Assignee:** developer

## Beschreibung
Tasks die seit ≥14 Tagen in der "Done"-Spalte liegen, sollen automatisch in die versteckte `_archive`-Spalte verschoben werden. Ein Background-Task (tokio::spawn) prüft alle 24h.

## Anforderungen
- [ ] Background-Task via `tokio::spawn` der alle 24h läuft
- [ ] Prüft alle Projekte: Tasks in "Done"-Spalte deren `updated_at` ≥ 14 Tage alt ist
- [ ] Verschiebt diese Tasks in die `_archive`-Spalte (column_id ändern)
- [ ] Setzt `previous_row` auf alte column_id
- [ ] Hängt Log-Eintrag an: "YYYY-MM-DD auto-archived"
- [ ] Fehler im Background-Task loggen (nicht paniken)

## Technische Hinweise
- `tokio::spawn` in `main()` vor `axum::serve()`
- `tokio::time::interval(Duration::from_secs(86400))` für 24h-Intervall
- Store braucht `Clone` (hat es bereits)
- "Done"-Spalte identifizieren über `title == "Done"` (case-sensitive)
- `_archive`-Spalte über `title == "_archive"` identifizieren
- `chrono` für Datums-Vergleich: `updated_at` parsen und mit `Utc::now() - Duration::days(14)` vergleichen

## Dev Log
- `archive_old_tasks()` Funktion implementiert: iteriert über alle Projekte, findet Done+Archive Spalten
- Prüft `updated_at` gegen 14-Tage-Cutoff via `chrono::DateTime::parse_from_rfc3339`
- Setzt `previous_row`, `column_id`, `updated_at` und hängt Log-Eintrag an
- Background-Task via `tokio::spawn` mit `tokio::time::interval(86400s)` in `main()`
- Fehler werden via `tracing::error!` geloggt, kein Panic

## Tester Notes
- Code-Review: sauber, Fehlerbehandlung korrekt (continue bei fehlenden Spalten, error-log bei Fehlern)
- Build: 0 errors, 0 warnings
- Alle 6 Anforderungen erfüllt

## Abnahme
