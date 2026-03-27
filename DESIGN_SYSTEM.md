# Plankton Design System

> Tailwind CSS v4 + shadcn-vue basiertes Design-System für die Plankton Kanban-App.

---

## 1. Farben (Dark Theme – Default)

| Token               | Hex       | Tailwind Class       | Verwendung                          |
|---------------------|-----------|----------------------|-------------------------------------|
| `bg`                | `#0e0e10` | `bg-bg`              | App-Hintergrund                     |
| `surface`           | `#18181c` | `bg-surface`         | Sidebar, Cards, Modals              |
| `surface-2`         | `#222228` | `bg-surface-2`       | Input-Hintergrund, verschachtelte Container |
| `border`            | `#2e2e38` | `border-border`      | Rahmen, Trennlinien                 |
| `accent`            | `#7c6af7` | `bg-accent`/`text-accent` | Primärfarbe (Buttons, Links, Focus) |
| `accent-dim`        | `#3d3570` | `bg-accent-dim`      | Accent-Background (Labels, Badges)  |
| `text`              | `#e2e2e8` | `text-text`          | Primärtext                          |
| `text-dim`          | `#7a7a8e` | `text-text-dim`      | Sekundärtext, Placeholder           |
| `danger`            | `#e05c6a` | `text-danger`/`border-danger` | Fehler, Löschen              |

### Badge-Farben

| Badge    | BG        | Text      | Border    |
|----------|-----------|-----------|-----------|
| Epic     | `#1a2e1a` | `#a5d6a7` | `#43a047` |
| Job      | `#1a2a3a` | `#90caf9` | `#1e88e5` |
| Blocked  | `#3a1c1c` | `#ff8a80` | `#e53935` |

### Light Theme Override (`body[data-theme="light"]`)

Alle Tokens werden via CSS Custom Properties überschrieben. Tailwind-Klassen bleiben identisch.

| Token | Dark | Light |
|-------|------|-------|
| `bg` | `#0e0e10` | `#f5f5f7` |
| `surface` | `#18181c` | `#ffffff` |
| `surface-2` | `#222228` | `#eeeef2` |
| `border` | `#2e2e38` | `#d8d8e0` |
| `accent` | `#7c6af7` | `#6b5ce7` |
| `accent-dim` | `#3d3570` | `#e8e4ff` |
| `text` | `#e2e2e8` | `#1a1a2e` |
| `text-dim` | `#7a7a8e` | `#6e6e82` |
| `danger` | `#e05c6a` | `#d94452` |

---

## 2. Typografie

| Rolle    | Font Family              | Tailwind  | Gewichte     |
|----------|--------------------------|-----------|--------------|
| Sans     | `IBM Plex Sans`          | `font-sans` | 300, 400, 600 |
| Mono     | `IBM Plex Mono`          | `font-mono` | 400, 600     |

### Schriftgrößen-Mapping

| Kontext          | CSS     | Tailwind        | Gewicht          |
|------------------|---------|-----------------|------------------|
| Detail Title     | 22px    | `text-[22px]`   | `font-bold`      |
| Logo             | 18px    | `text-lg`       | `font-semibold`  |
| Board Title      | 16px    | `text-base`     | `font-semibold`  |
| Body/Input       | 14px    | `text-sm`       | `font-normal`    |
| Card Title       | 13px    | `text-[13px]`   | `font-semibold`  |
| Button           | 13px    | `text-[13px]`   | `font-semibold`  |
| Section Header   | 11px    | `text-[11px]`   | `font-semibold`  |
| Badge/Label      | 10px    | `text-[10px]`   | `font-semibold`  |
| Type Badge       | 9px     | `text-[9px]`    | `font-bold`      |

### Section Title Pattern
```html
<span class="font-mono text-[11px] font-semibold uppercase tracking-wider text-text-dim">
  SECTION TITLE
</span>
```

---

## 3. Spacing & Radii

### Border Radius

| Token        | Wert   | Tailwind        | Verwendung              |
|-------------|--------|-----------------|-------------------------|
| `radius-sm` | 3px    | `rounded-sm`    | Tags, kleine Badges     |
| `radius-md` | 6px    | `rounded-md`    | Buttons, Inputs, Cards  |
| `radius-lg` | 10px   | `rounded-lg`    | Modals, große Container |
| `radius-full`| 9999px| `rounded-full`  | Avatare, Pills          |

---

## 4. Komponenten (shadcn-vue)

### Button Varianten

| Variante  | Tailwind-Klassen |
|-----------|------------------|
| `default` | `bg-accent text-white font-semibold rounded-md px-5 py-2 text-[13px] hover:opacity-85 transition-opacity` |
| `danger`  | `bg-transparent border border-danger text-danger rounded-md px-5 py-2 text-[13px] hover:bg-danger/10` |
| `outline` | `bg-transparent border border-border text-text-dim rounded-md px-2 py-1 text-sm hover:border-accent hover:text-accent transition-all` |
| `ghost`   | `bg-transparent text-text-dim hover:bg-surface-2 rounded-md px-2 py-1` |
| `sm`      | Zusatz: `px-3.5 py-1.5 text-xs` |

### Badge Varianten

| Variante  | Klassen |
|-----------|---------|
| `default` | `bg-accent-dim border border-accent text-accent font-mono text-[10px] px-1.5 py-px rounded-sm` |
| `epic`    | `bg-badge-epic-bg border border-badge-epic-border text-badge-epic-text` |
| `job`     | `bg-badge-job-bg border border-badge-job-border text-badge-job-text` |
| `blocked` | `bg-badge-blocked-bg border border-badge-blocked-border text-badge-blocked-text` |

### Input
```html
<input class="w-full bg-surface-2 border border-border rounded-md text-text text-sm
  px-3 py-2 font-sans outline-none transition-colors
  focus:border-accent placeholder:text-text-dim" />
```

### Modal/Dialog
- Overlay: `fixed inset-0 bg-black/70 backdrop-blur-[2px] z-[1000] flex items-center justify-center`
- Panel: `bg-surface border border-border rounded-lg shadow-[0_16px_48px_rgba(0,0,0,0.5)] max-w-[480px] w-[90%] p-6 flex flex-col gap-3.5`
- Wide: `max-w-[1000px]`
- Detail: `max-w-[1440px] max-h-[90vh] overflow-y-auto`

---

## 5. Layout

### App Shell
```
flex h-screen overflow-hidden
├── aside (w-[220px] min-w-[220px] bg-surface border-r border-border flex flex-col)
└── main (flex-1 flex flex-col overflow-hidden)
    ├── header (px-6 py-4 border-b border-border bg-surface flex items-center gap-3)
    └── div.board (flex-1 overflow-x-auto overflow-y-hidden p-5 px-6)
```

### Kanban Column
```
min-w-[280px] max-w-[320px] flex-[0_0_300px] bg-surface rounded-lg
flex flex-col max-h-[calc(100vh-140px)]
```

### Responsive Breakpoints
| Breakpoint | Änderungen |
|------------|-----------|
| Desktop (>1024px) | Standard-Layout |
| Tablet (≤1024px) | Schmalere Spalten 260px |
| Mobile (≤768px) | Sidebar-Drawer, Fullscreen-Modals, 16px Inputs |
| Small (≤480px) | Spalten 85vw, kompakte Header |

---

## 6. Animationen

| Name         | Klasse/Keyframe     | Dauer | Verwendung |
|--------------|---------------------|-------|------------|
| `task-glow`  | `animate-task-glow` | 2s    | Neuer Task Highlight |
| `git-pulse`  | `animate-git-pulse` | 2s    | Git-Error Indikator |
| Transitions  | `transition-all duration-150` | 150ms | Hover/Focus |
| Theme Switch | `transition-colors duration-250` | 250ms | Theme Toggle |

---

## 7. Icon-System

**Primär:** Lucide Vue Next (`lucide-vue-next`)
- Menu, X, Search, Plus, Settings, LogOut, Key, ChevronDown, ChevronRight, etc.

**Migration von Unicode:**
| Alt | Neu (Lucide) |
|-----|--------------|
| ☰ `&#9776;` | `<Menu />` |
| ✕ `&#10005;` | `<X />` |
| 🔍 `&#128269;` | `<Search />` |
| 🔑 `&#128273;` | `<Key />` |
| ⚙ `&#9881;` | `<Settings />` |
| ⏻ `&#9211;` | `<LogOut />` |
| ☀ `&#9728;` | `<Sun />` / `<Moon />` |

---

## 8. Dateistruktur

```
src/frontend/
├── styles/
│   └── globals.css              # Tailwind v4 @theme + @layer base/components
├── lib/
│   └── utils.ts                 # cn() utility (clsx + twMerge)
├── components/
│   ├── ui/                      # shadcn-vue Basis-Komponenten
│   │   ├── Button.vue
│   │   ├── Badge.vue
│   │   ├── Input.vue
│   │   ├── Textarea.vue
│   │   ├── Dialog.vue
│   │   ├── Select.vue
│   │   ├── Separator.vue
│   │   └── Sheet.vue
│   ├── App.vue                  # Root (Auth + Login)
│   ├── AppLayout.vue            # Shell (Sidebar + Header + Modals)
│   ├── KanbanBoard.vue          # Board + Task Cards
│   ├── TaskModal.vue            # Task Create/Edit
│   ├── TaskDetail.vue           # Task Read-Only Detail
│   └── ImportPage.vue           # Mobile Import
└── composables/
    └── useTheme.ts              # Dark/Light Mode
```
