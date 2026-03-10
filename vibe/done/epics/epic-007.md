# Epic-007: Dark/Light Mode Toggle

## Status: done

## Beschreibung

Plankton hat derzeit ausschließlich ein Dark Theme. Diese Epic fügt einen
Theme-Toggle hinzu, mit dem zwischen Dark Mode und Light Mode gewechselt
werden kann. Die Auswahl wird im localStorage persistiert.

## Anforderungen

### CSS

- Light-Theme als alternative CSS-Custom-Properties definieren
  (--bg, --surface, --surface-2, --border, --text, --text-dim, etc.)
- Theme-Klasse am `<html>` oder `<body>` Element (z.B. `data-theme="light"`)
- Smooth Transition beim Wechsel (color/background Transition)

### Frontend

- Toggle-Button in der Sidebar oder im Board-Header
- Icon wechselt je nach aktivem Theme (Sonne/Mond)
- Theme-Auswahl wird im localStorage gespeichert
- Beim Laden: gespeichertes Theme anwenden, Fallback: Dark Mode
- Optional: System-Präferenz respektieren (prefers-color-scheme)

### Umsetzung

- Keine Backend-Änderungen nötig
- Rein CSS + JS Implementierung

## Tasks

- [ ] Task-019: Light-Theme CSS-Variablen definieren
- [ ] Task-020: Theme-Toggle Button & Switch-Logik (JS)
- [ ] Task-021: localStorage-Persistierung & System-Präferenz

## Quelle

/flow/ideas/dark-mode.md
