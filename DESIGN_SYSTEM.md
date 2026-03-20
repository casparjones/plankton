# Plankton Design System

Derived from the actual codebase. This document is the canonical reference for visual decisions.

---

## Tokens

### Colors

| Token | Dark (default) | Light | Usage |
|-------|---------------|-------|-------|
| `--bg` | `#0e0e10` | `#f5f5f7` | Page background |
| `--surface` | `#18181c` | `#ffffff` | Cards, sidebar, modals |
| `--surface-2` | `#222228` | `#eeeef2` | Inputs, nested surfaces |
| `--border` | `#2e2e38` | `#d8d8e0` | All borders |
| `--accent` | `#7c6af7` | `#6b5ce7` | Primary action, links, focus |
| `--accent-dim` | `#3d3570` | `#e8e4ff` | Accent backgrounds (badges) |
| `--text` | `#e2e2e8` | `#1a1a2e` | Primary text |
| `--text-dim` | `#7a7a8e` | `#6e6e82` | Secondary text, labels |
| `--danger` | `#e05c6a` | `#d94452` | Errors, delete actions |

**Semantic colors (hardcoded, dark-only):**

| Name | Background | Border | Text | Usage |
|------|-----------|--------|------|-------|
| success | `#1a2e1a` | `#43a047` | `#a5d6a7` | Done, epic badge, subtask check |
| info | `#1a2a3a` | `#1e88e5` | `#90caf9` | Job badge, enhancement label |
| warning | `#3a2e1a` | `#fb8c00` | `#ffcc80` | Review label |
| error | `#3a1c1c` | `#e53935` | `#ff8a80` | Blocked badge, validation errors |
| purple | `#2a1a3a` | `#8e24aa` | `#ce93d8` | Design/UI labels |
| teal | `#1a3a3a` | `#00897b` | `#80cbc4` | Docs labels |
| yellow | `#2a2a1a` | `#fdd835` | `#fff59d` | Refactor label |

> **Rule:** Theme tokens use `var()`. Semantic status colors are hardcoded in `utils.ts:labelColor()` and board.css because they are dark-theme-specific decorative accents, not structural.

### Spacing

No spacing variables exist. Values are hardcoded. The recurring scale is:

| Step | Value | Common usage |
|------|-------|-------------|
| xs | `2px` | Inline gaps |
| sm | `4px` | Tight gaps, small padding |
| md | `6px` | Label gaps, list gaps |
| base | `8px` | Standard gap, small padding |
| lg | `10-12px` | Input padding, section gaps |
| xl | `14-16px` | Card padding, section padding |
| 2xl | `20px` | Section padding, board margins |
| 3xl | `24px` | Board padding, modal gaps |
| 4xl | `40px` | Login card padding |

> **Rule:** Use literal px values from this scale. Do not invent new spacing tokens — the codebase uses explicit values intentionally.

### Typography

**Fonts:**
- `var(--font-sans)`: IBM Plex Sans (300, 400, 600) — body text, inputs
- `var(--font-mono)`: IBM Plex Mono (400, 600) — labels, code, badges, titles

**Base:** `font-size: 14px`, `line-height: 1.5` on `body`.

| Category | Size | Weight | Font | Usage |
|----------|------|--------|------|-------|
| Page title | `22px` | 700 | sans | Task detail title |
| Section title | `16-18px` | 600 | mono | Board title, sidebar logo |
| Body | `14px` | 400 | sans | Default text, inputs |
| Small body | `13px` | 400 | sans/mono | Comments, descriptions, buttons |
| Label | `11-12px` | 600 | mono | Section headers, info labels |
| Badge | `10px` | 400 | mono | Labels, tags, column counts |
| Micro | `9px` | 700 | mono | Type badges (E/J/B) |

**Letter spacing:**
- `0.02em` — titles
- `0.04em` — labels, logo
- `0.06em` — uppercase section headers

**Line heights:** `1.3` (headings), `1.4` (compact lists), `1.5` (body), `1.6` (descriptions)

### Elevation / Shadows

| Level | Value | Usage |
|-------|-------|-------|
| 0 | none | Default |
| 1 | `0 2px 12px rgba(124, 106, 247, 0.15)` | Task card hover |
| 2 | `0 8px 24px rgba(0,0,0,0.4)` | Context menus, dropdowns |
| 3 | `0 16px 48px rgba(0,0,0,0.5)` | Modal overlay |
| sidebar | `4px 0 20px rgba(0, 0, 0, 0.4)` | Mobile sidebar drawer |

### Border Radius

| Token/Value | Usage |
|-------------|-------|
| `var(--radius)` = `6px` | Standard: inputs, cards, buttons, modals |
| `3px` | Small: labels, badges, code blocks, type badges |
| `8px` | Medium: login inputs, detail badges |
| `12px` | Large: detail label pills |
| `16px` | Extra: login card |
| `50%` | Circle: avatars, color swatches |

> **Rule:** Use `var(--radius)` for standard elements. Use `3px` for small decorative badges. Use `50%` only for circles.

---

## Themes

Switching via `body[data-theme="light"]`. Default is dark.

### Dark (default)
- Background progression: `#0e0e10` → `#18181c` → `#222228` (bg → surface → surface-2)
- Text: white-ish (`#e2e2e8`) with dim grey (`#7a7a8e`)
- Accent: purple (`#7c6af7`)

### Light
- Background progression: `#f5f5f7` → `#ffffff` → `#eeeef2`
- Text: dark (`#1a1a2e`) with grey (`#6e6e82`)
- Accent: slightly darker purple (`#6b5ce7`)

### Login page
The login page uses its own color scheme (ocean dark, cyan accent `#18f9f5`) which is intentionally disconnected from the app theme. This is a branded splash screen, not part of the design system.

---

## Components

### Button

```html
<button class="btn-primary">Primary Action</button>
<button class="btn-danger">Destructive</button>
<button class="btn-small">Small</button>
```

- `font-size: 13px`, `padding: 8px 20px`, `border-radius: var(--radius)`
- Primary: `background: var(--accent)`, `color: #fff`
- Ghost/secondary: `background: none`, `border: 1px solid var(--border)`, `color: var(--text-dim)`
- Hover: `border-color: var(--accent)`, `color: var(--accent)`
- **Do:** Use `btn-primary` for the main action in a modal. One per view.
- **Don't:** Use hardcoded colors on buttons.

### Label / Badge

```html
<span class="label" style="background: ...; border-color: ...; color: ...">feature</span>
```

- `font-family: var(--font-mono)`, `font-size: 10px`, `padding: 1px 6px`, `border-radius: 3px`
- Colors come from `labelColor()` in `utils.ts` — content-based coloring
- **Do:** Use `labelColor()` for dynamic styling.
- **Don't:** Hardcode label colors in templates.

### Type Badge

```html
<span class="type-badge type-epic">E</span>
<span class="type-badge type-job">J</span>
<span class="blocked-badge">B</span>
```

- `9px`, `18x18px`, `border-radius: 3px`, monospace bold
- Epic: green, Job: blue, Blocked: red

### Card (Task)

```html
<div class="task-inner" style="border-left: 3px solid {workerColor}">
  <div class="task-header-row">...</div>
  <div class="task-desc">...</div>
  <div class="task-meta">...</div>
</div>
```

- `background: var(--surface)`, `border: 1px solid var(--border)`, `border-radius: var(--radius)`
- Left border colored by worker hash
- Hover: `border-color: var(--accent)`, shadow level 1
- **Do:** Keep card content minimal (title, description snippet, labels, avatar).
- **Don't:** Add more than 3 lines of description preview.

### Modal

```html
<div class="modal-overlay open">
  <div class="modal modal-detail">
    <div class="modal-header">...</div>
    <!-- content -->
    <div class="modal-actions">...</div>
  </div>
</div>
```

- Overlay: `rgba(0, 0, 0, 0.7)`, `z-index: 1000`
- Modal: `background: var(--surface)`, `border: 1px solid var(--border)`, `border-radius: 8px`
- Shadow: level 3
- Mobile: fullscreen (`width: 100%`, `border-radius: 0`)
- **Do:** Place primary action button rightmost in `.modal-actions`.
- **Don't:** Nest modals.

### Input / Textarea / Select

```html
<input class="..." type="text" />
<textarea rows="4"></textarea>
<select>...</select>
```

- `background: var(--surface-2)`, `border: 1px solid var(--border)`, `border-radius: var(--radius)`
- `color: var(--text)`, `font-size: 14px`, `padding: 10px 12px`
- Focus: `border-color: var(--accent)`
- Placeholder: `color: var(--text-dim)`
- Mobile: `font-size: 16px !important` (prevents iOS zoom)
- **Do:** Always use `var(--surface-2)` for input backgrounds.
- **Don't:** Use `outline` for focus — use `border-color` change.

### Section Header

```html
<span class="detail-section-title">KOMMENTARE</span>
```

- `font-family: var(--font-mono)`, `font-size: 11px`, `font-weight: 600`
- `text-transform: uppercase`, `letter-spacing: 0.06em`, `color: var(--text-dim)`
- `border-bottom: 1px solid var(--border)`, `padding-bottom: 4px`

### Avatar

```html
<span class="avatar" title="Frank">F</span>
```

- `border-radius: 50%`, `background: var(--surface)`, `border: 1px solid var(--border)`
- Single uppercase letter, monospace

---

## Breakpoints

| Name | Width | Changes |
|------|-------|---------|
| Desktop | > 1024px | Full layout: sidebar + board |
| Tablet | ≤ 1024px | Narrower columns (260px) |
| Mobile | ≤ 768px | Sidebar → drawer, modals fullscreen, 16px inputs |
| Small phone | ≤ 480px | Columns 85vw, compact header, import button hidden |

Additional: `≤ 900px` for task detail grid → single column.

---

## Z-Index Layers

| Layer | Z-Index | Usage |
|-------|---------|-------|
| Sticky headers | `10` | Search bar, import header |
| Sidebar overlay | `999` | Dark backdrop on mobile |
| Sidebar drawer | `1000` | Sidebar on mobile |
| Modals | `1000` | All modal overlays |
| Context menus | `2000` | Column menus, dropdowns |

---

## Conventions & Rules

### What belongs in a CSS variable
- **Yes:** Colors that change with theme, border-radius, font families
- **No:** Spacing values, font sizes, shadows, z-indexes — these are explicit

### Naming
- CSS classes: `kebab-case` (`task-inner`, `board-header`, `detail-section-title`)
- Modifiers via extra class: `col-done`, `sidebar-open`, `type-epic`
- State: `.open`, `.done`, `.dragging`
- No BEM, no utility classes

### When to create a component vs inline
- 3+ usages → extract a class
- 1 usage → inline or scoped to parent
- Template-level `style` bindings OK for dynamic values (colors, widths)

### Transitions
- UI interactions: `0.15s` (hover, focus, border changes)
- Theme switching: `0.25s ease`
- Sidebar drawer: `0.25s ease`
- No easing function needed for < 0.2s transitions

### Color application
- Structural colors → `var(--token)`
- Status/label decorative colors → hardcoded from `labelColor()` map
- Opacity overlays → `rgba(0, 0, 0, opacity)` with `0.4` (shadow), `0.5` (overlay), `0.7` (modal)

---

## Known Violations

### ❌ Hard Violations

| File | Issue |
|------|-------|
| `login.css` | Entire login page uses its own color scheme (`#0a1a20`, `#18f9f5`) disconnected from theme variables. Works as branded splash but won't adapt to light theme. |
| `board.css:193` | `.label` base style uses `var(--accent-dim)` / `var(--accent)` but is always overridden by inline `style` from `labelColor()`. Dead CSS. |
| `import.css:3-13` | `.import-btn` defined twice — once for modal (line 3) and once for mobile page (line 209). Specificity conflict resolved by `.import-page .import-btn` but fragile. |

### ⚠️ Minor Deviations

| File | Issue |
|------|-------|
| `board.css` | Mixes `!important` extensively on kanban columns. Legacy from jKanban override — acceptable but not ideal. |
| `task-detail.css:148` | `.detail-label` has `border-radius: 12px` (pill shape) while all other badges use `3px`. Intentional distinction for detail view. |
| `modals.css` | Modal widths vary: `modal-wide` has no explicit width, `modal-detail` has none. Rely on content sizing which occasionally overflows on tablet. |
| `responsive.css` | Breakpoint at `900px` (task-detail.css) doesn't match the standard set (480/768/1024). One-off for grid layout. |
| Multiple files | `font-size: 13px` used ~20 times but never as a variable. Consider canonizing if a spacing token system is ever introduced. |

### ✅ Conforming Patterns

- All structural colors consistently use `var()` tokens
- `var(--radius)` used correctly on all standard elements
- Font families always via `var(--font-mono)` / `var(--font-sans)`
- Theme toggle works correctly via `data-theme` attribute
- Transitions consistent at `0.15s` for interactions
- Mobile breakpoints properly applied with touch-friendly targets
- Input focus states consistently use `border-color: var(--accent)`
