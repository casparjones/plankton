# Task-019: Light-Theme CSS-Variablen definieren

**Epic:** epic-007
**Status:** open
**Rolle:** Developer

## Beschreibung
Light-Theme CSS-Custom-Properties als Alternative zum bestehenden Dark-Theme definieren. Theme-Klasse `data-theme="light"` am `<body>` Element. Smooth Transitions beim Wechsel.

## Akzeptanzkriterien
- [ ] Light-Theme Variablen in styles.css definiert (--bg, --surface, --surface-2, --border, --text, --text-dim, --accent, --accent-dim, --danger)
- [ ] `body[data-theme="light"]` überschreibt alle relevanten Variablen
- [ ] Transition auf body für smooth Farbwechsel
- [ ] Alle UI-Elemente sehen im Light-Theme korrekt aus (Kontrast, Lesbarkeit)
