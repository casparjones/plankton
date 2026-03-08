# Idee: Git-Repository-Integration für Projekte

## Kernidee

Jedes Plankton-Projekt kann optional mit einem Git-Repository verknüpft werden.
Bei Änderungen am Board (Tasks, Spalten, Projektdaten) wird automatisch ein Commit
in das hinterlegte Repository gepusht. So hat man eine vollständige, versionierte
Historie aller Board-Änderungen direkt in Git – nachvollziehbar, exportierbar,
und unabhängig von Plankton selbst wiederherstellbar.

## Was gepusht wird

Das Projekt wird als einzelne JSON-Datei im Repository gespeichert:
`plankton/<project-id>.json` oder ein konfigurierbarer Pfad.

Das Format ist das bestehende flache ProjectDoc-Format – also direkt
importierbar zurück in Plankton.

## Konfiguration pro Projekt

In den Projekteinstellungen kann der Admin hinterlegen:
- Repository-URL (SSH oder HTTPS)
  z.B. git@github.com:frank/mein-projekt.git
- Branch (default: main)
- Pfad innerhalb des Repos (default: plankton/<project-title>.json)
- SSH-Key oder Access-Token für die Authentifizierung (verschlüsselt gespeichert)
- Commit-Autor Name + Email (z.B. "Plankton Bot" / plankton@lovely-apps.de)
- Auto-Push: an/aus (wenn aus: nur manueller Push-Button im Frontend)

## Wann wird gepusht

- Bei jeder Änderung am Projekt (Task anlegen, verschieben, editieren, löschen)
- Bei Spalten-Änderungen
- Commit-Message enthält automatisch den Auslöser und den Nutzer:
  "chore: Task 'Feature X' moved to In Progress [Frank]"
  "chore: Task 'Bug Fix' created [Claude Developer]"
  "chore: Column 'Review' added [Frank]"
  "chore: Task 'Old Feature' archived [Plankton Auto-Archive]"

## Technische Umsetzung

**Backend (Rust):**
- Rust-Crate: `git2` (libgit2-Bindings, kein Shell-Aufruf nötig)
- Nach jedem erfolgreichen put_project()-Aufruf wird async ein Git-Push getriggert
- Der Push läuft in einem tokio::spawn um den API-Response nicht zu blockieren
- Bei Push-Fehler: Fehler wird geloggt, API-Response bleibt trotzdem 200
  (Git-Fehler sollen die App nicht blockieren)
- Push-Status wird im Projekt gespeichert:
  last_git_push: { status: "success" | "failed", message: String, timestamp: String }

**Authentifizierung:**
- SSH-Key: wird verschlüsselt in der DB gespeichert, nur für git2 entschlüsselt
- HTTPS-Token: wird als Credential in der Repository-URL oder separat gespeichert
- Beide Varianten werden unterstützt

**Konfliktstrategie Push:**
- Plankton ist immer der Single Source of Truth beim normalen Betrieb
- Bei Push-Konflikt: force-push auf den konfigurierten Branch

**Pull / Sync (manuell, auf Knopfdruck):**
- Einmaliger Init-Pull: beim ersten Verbinden eines Repos kann der Nutzer
  die JSON-Datei aus dem Repository als Startzustand in Plankton laden
- Manueller Sync-Pull: jederzeit auf Knopfdruck auslösbar um den aktuellen
  Stand aus Git zu holen und das Board zu überschreiben
- Ablauf eines Pull/Sync:
    1. git fetch vom Remote
    2. Lokalen Stand mit Remote vergleichen (Fast-Forward-Check)
    3. Gibt es einen Konflikt (Remote und Plankton haben divergierende Historien)?
       → Sofortiger Abbruch, Fehlermeldung im Frontend, keine Änderung an den Daten
    4. Kein Konflikt (Remote ist ahead, reine Fast-Forward-Situation)?
       → JSON-Datei aus dem Remote-Branch lesen
       → Plankton-Projekt wird mit dem gelesenen Inhalt überschrieben
       → Bestätigungsmeldung: "Sync erfolgreich – Board wurde auf Git-Stand gebracht"
- Ein Konflikt liegt vor wenn: Plankton lokale Commits hat die nicht im Remote sind
  UND der Remote Commits hat die nicht in Plankton sind (echter Divergenz-Fall)
- Kein automatischer Merge, kein Rebase – bei Konflikt wird immer abgebrochen

## Frontend

**Projekteinstellungen (neuer Tab "Git"):**
- Toggle: Git-Integration aktiv/inaktiv
- Felder: Repository-URL, Branch, Dateipfad, Commit-Autor, Auth-Methode
- SSH-Key Upload oder Token-Eingabe (masked)
- Verbindung testen Button → prüft ob Push möglich ist
- Status-Anzeige: letzter Push (Zeitpunkt + Erfolg/Fehler)
- Manueller Push-Button (auch wenn Auto-Push deaktiviert)

**Board-Header:**
- Kleines Git-Icon mit Status-Indikator:
  grün = letzter Push erfolgreich
  rot = letzter Push fehlgeschlagen (Tooltip mit Fehlermeldung)
  grau = Git nicht konfiguriert

## Frontend-Ergänzungen für Pull/Sync

**Projekteinstellungen Git-Tab (Ergänzung):**
- "Init from Git" Button: nur sichtbar wenn das Projekt noch leer ist oder
  explizit zurückgesetzt werden soll – lädt den Git-Stand als Startzustand
- "Sync from Git" Button: überschreibt das Board mit dem aktuellen Git-Stand,
  zeigt vorher einen Bestätigungs-Dialog:
  "Das Board wird mit dem Stand aus Git überschrieben. Fortfahren?"
- Bei Konflikt-Abbruch: roter Hinweis mit Erklärung:
  "Sync abgebrochen: Lokale und Remote-Historie haben divergiert.
  Bitte Push durchführen oder Repository manuell bereinigen."
- Sync-Status: letzter Pull (Zeitpunkt + Erfolg/Fehler) neben dem Push-Status

## Mögliche Erweiterung (nicht im ersten Epic)

- Webhook: bei Git-Push von außen automatischen Sync auslösen (bidirektional)
- Mehrere Repositories pro Projekt (z.B. Board-State + Code-Repo getrennt)
- Diff-Ansicht: was hat sich seit dem letzten Sync verändert?