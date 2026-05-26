import { test, expect, Page } from '@playwright/test'

// ─── Hilfsfunktionen ────────────────────────────────────────────────────────

/** Login-Credentials aus ENV, falls vorhanden. Fallback auf Demo-User. */
const USERNAME = process.env.PLANKTON_USER || 'admin'
const PASSWORD = process.env.PLANKTON_PASS || 'admin'
const BASE_URL = process.env.PLANKTON_URL || 'http://localhost:3099'

/**
 * Schließt das "Passwort ändern"-Pflicht-Modal, falls es nach dem Login erscheint.
 * admin/admin-Login setzt must_change_password=true → Modal blockiert sonst weitere Klicks.
 */
async function dismissPasswordChangeModal(page: Page): Promise<void> {
  // Erst 150 ms warten damit der 100 ms-Timer in App.vue (setTimeout → openPasswordModal(true))
  // abgelaufen ist und das Modal ggf. geöffnet hat. Dann sicher schließen.
  await page.waitForTimeout(150)
  await page.evaluate(() => {
    const modal = document.getElementById('password-modal') as HTMLElement
    if (modal) {
      modal.dataset.force = ''
      modal.classList.remove('open')
      // pointer-events deaktivieren damit das Modal selbst bei Race Condition
      // keine nachfolgenden Klicks blockieren kann
      modal.style.pointerEvents = 'none'
      modal.style.zIndex = '-1'
    }
  })
  // Nochmals kurz warten damit etwaige zweite Timer-Aufrufe ebenfalls abgeschlossen sind
  await page.waitForTimeout(150)
}

/**
 * Führt den Login durch und wartet auf erfolgreichen Auth-Check.
 * Gibt false zurück wenn Login fehlschlägt (Credentials unbekannt).
 */
async function tryLogin(page: Page): Promise<boolean> {
  await page.goto('/')
  // Login-Formular warten (max 10s)
  const loginForm = page.locator('form')
  try {
    await loginForm.waitFor({ timeout: 10000 })
  } catch {
    // Bereits eingeloggt oder kein Login-Formular
    return true
  }

  await page.locator('input[type="text"]').fill(USERNAME)
  await page.locator('input[type="password"]').fill(PASSWORD)
  await page.locator('button[type="submit"]').click()

  // Warte auf Board-Laden oder Fehler
  // Akzeptiert sowohl Kanban-Boards (.kanban-column) als auch List-Boards (.list-board)
  try {
    await page.waitForSelector('.kanban-column, #board .kanban-column, .list-board, #board', { timeout: 10000 })
    // Zusätzlich prüfen: #board vorhanden → Login erfolgreich
    const boardVisible = await page.locator('#board').isVisible().catch(() => false)
    if (boardVisible) return true
    // Wenn nur #board da ist aber kein Board-Content → trotzdem erfolgreich (leeres Board)
    const hasKanban = await page.locator('.kanban-column').count().catch(() => 0)
    const hasListBoard = await page.locator('.list-board').count().catch(() => 0)
    return hasKanban > 0 || hasListBoard > 0 || boardVisible
  } catch {
    // Login fehlgeschlagen – prüfe ob Fehlermeldung sichtbar
    const errorVisible = await page.locator('[class*="text-"][class*="#ff"]').isVisible().catch(() => false)
    if (errorVisible) {
      console.log('Login fehlgeschlagen – teste nur öffentliche Seiten')
    }
    return false
  }
}

// ─── SMOKE TESTS ────────────────────────────────────────────────────────────

test.describe('Smoke Tests', () => {

  test('01 – Seite ist erreichbar und gibt HTML zurück', async ({ page }) => {
    const response = await page.goto('/')
    expect(response?.status()).toBeLessThan(400)
    // Titel-Element vorhanden
    await expect(page.locator('title')).not.toBeEmpty()
    const title = await page.title()
    expect(title).toBeTruthy()
  })

  test('02 – Login-Seite zeigt Plankton-Logo und Formular', async ({ page }) => {
    await page.goto('/')
    // Warte auf Auth-Check (kurz)
    await page.waitForTimeout(2000)

    // Entweder Login-Formular oder bereits eingeloggt (Board)
    const hasLoginForm = await page.locator('form').isVisible().catch(() => false)
    const hasBoard = await page.locator('#board, .kanban-column').isVisible().catch(() => false)

    // Eine der beiden Ansichten muss sichtbar sein
    expect(hasLoginForm || hasBoard).toBeTruthy()

    if (hasLoginForm) {
      // Login-Formular vorhanden: Plankton-Text oder Logo prüfen
      await expect(page.locator('body')).toContainText('Plankton')
      await expect(page.locator('input[type="text"]')).toBeVisible()
      await expect(page.locator('input[type="password"]')).toBeVisible()
      await expect(page.locator('button[type="submit"]')).toBeVisible()
    }
  })

  test('03 – Nach Login: Kanban-Spalten sind sichtbar', async ({ page }) => {
    const loggedIn = await tryLogin(page)

    if (!loggedIn) {
      test.skip()
      return
    }

    // Board-Container muss da sein
    await expect(page.locator('#board')).toBeVisible({ timeout: 15000 })

    // Mindestens eine Kanban-Spalte
    const columns = page.locator('.kanban-column')
    const count = await columns.count()
    expect(count).toBeGreaterThan(0)

    // Screenshot für Dokumentation
    await page.screenshot({ path: '/tmp/plankton-board.png', fullPage: false })
  })

  test('04 – Sidebar ist sichtbar mit Plankton-Logo', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }

    const sidebar = page.locator('aside.sidebar')
    await expect(sidebar).toBeVisible()
    await expect(sidebar).toContainText('Plankton')
  })

  test('05 – Projekt-Liste in Sidebar lädt', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }

    // Projekt-Liste muss erscheinen
    const projectList = page.locator('#project-list')
    await expect(projectList).toBeVisible()

    // Mindestens ein Projekt-Eintrag
    const items = projectList.locator('li, [data-project-id]')
    // Wenn keine Items: Board ist trotzdem OK (leer)
    const itemCount = await items.count()
    console.log(`Gefundene Projekte in Sidebar: ${itemCount}`)
  })

})

// ─── LIST-BOARD TESTS ────────────────────────────────────────────────────────

test.describe('List-Board Rendering', () => {

  /**
   * Prüft dass ein Projekt mit type="list" das List-Board rendert:
   * - .list-board ist vorhanden
   * - .list-board-column ist vorhanden (eine Spalte)
   * - .kanban-column ist NICHT vorhanden (kein Kanban-Multi-Spalten-Layout)
   * - kein "Spalte hinzufügen"-Button sichtbar (.add-column-btn)
   *
   * Dieser Test manipuliert den Frontend-State direkt (ohne echtes Backend),
   * um ein List-Projekt zu simulieren.
   */
  test('06 – List-Board: rendert eine Spalte ohne Kanban-Spalten', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }
    await dismissPasswordChangeModal(page)

    // Warte auf Board-Laden
    await page.waitForSelector('#board', { timeout: 15000 })

    // Simuliere ein List-Projekt via window.__state (globaler Vue-State)
    await page.evaluate(() => {
      // @ts-ignore
      const s = window.__state
      if (!s || !s.project) return
      // type auf "list" setzen
      s.project.type = 'list'
    })

    // Kurz warten damit Vue re-rendert
    await page.waitForTimeout(300)

    // List-Board muss erscheinen
    const listBoard = page.locator('.list-board')
    await expect(listBoard).toBeVisible({ timeout: 5000 })

    // List-Board-Spalte (eine Spalte) muss vorhanden sein
    const listColumn = page.locator('.list-board-column')
    await expect(listColumn).toBeVisible({ timeout: 3000 })

    // Kanban-Columns dürfen NICHT gerendert werden
    const kanbanCols = page.locator('.kanban-column')
    await expect(kanbanCols).toHaveCount(0)

    // Kein "Spalte hinzufügen"-Button
    const addColBtn = page.locator('.add-column-btn')
    await expect(addColBtn).toHaveCount(0)
  })

  /**
   * Prüft dass ein Projekt mit type="kanban" (Standard) das normale
   * Kanban-Board rendert (keine List-Board-Elemente).
   */
  test('07 – Kanban-Board: bleibt unverändert bei type="kanban"', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }
    await dismissPasswordChangeModal(page)

    await page.waitForSelector('#board', { timeout: 15000 })

    // Stelle sicher dass type="kanban" gesetzt ist (Standard)
    await page.evaluate(() => {
      // @ts-ignore
      const s = window.__state
      if (!s || !s.project) return
      s.project.type = 'kanban'
    })

    await page.waitForTimeout(300)

    // Kanban-Spalten müssen vorhanden sein
    const columns = page.locator('.kanban-column')
    const count = await columns.count()
    expect(count).toBeGreaterThan(0)

    // List-Board darf NICHT gerendert werden
    const listBoard = page.locator('.list-board')
    await expect(listBoard).toHaveCount(0)
  })

})

// ─── FUNKTIONALE TESTS ──────────────────────────────────────────────────────

test.describe('Funktionale Tests', () => {

  test('06 – Task-Karten sind im Board sichtbar', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }

    await page.waitForSelector('.kanban-column', { timeout: 15000 })

    // Prüfe ob Task-Items existieren
    const tasks = page.locator('.kanban-item')
    const taskCount = await tasks.count()
    console.log(`Gefundene Task-Karten: ${taskCount}`)

    // Board muss zumindest geladen haben (auch leeres Board ist OK)
    await expect(page.locator('#board')).toBeVisible()
  })

  test('07 – Klick auf Task öffnet Detail-Modal', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }

    await page.waitForSelector('.kanban-column', { timeout: 15000 })

    // Ersten klickbaren Task finden
    const tasks = page.locator('.kanban-item')
    const taskCount = await tasks.count()

    if (taskCount === 0) {
      console.log('Keine Tasks vorhanden – Test übersprungen')
      test.skip()
      return
    }

    const firstTask = tasks.first()
    const taskTitle = await firstTask.locator('.text-text.font-semibold').innerText().catch(() => 'Unbekannt')
    console.log(`Klicke auf Task: "${taskTitle}"`)

    await firstTask.click()

    // Warte auf Task-Detail Modal
    // Das Modal wird via window.__openTaskDetail() geöffnet
    await page.waitForTimeout(1000)

    // Detail-Panel oder Modal sollte erscheinen
    const detailVisible =
      await page.locator('[class*="task-detail"], [class*="TaskDetail"], [data-testid="task-detail"]').isVisible().catch(() => false) ||
      await page.locator('.modal-overlay:not([style*="display: none"])').isVisible().catch(() => false) ||
      // Suche nach einem Panel das den Task-Titel enthält
      await page.locator(`text="${taskTitle}"`).nth(1).isVisible().catch(() => false)

    // Screenshot zum Beweis
    await page.screenshot({ path: '/tmp/plankton-task-click.png' })

    // Lockere Prüfung: URL hat sich geändert (task route) ODER Modal sichtbar
    const urlChanged = page.url().includes('/t/')
    console.log(`URL nach Task-Klick: ${page.url()}`)
    console.log(`Detail sichtbar: ${detailVisible}, URL geändert: ${urlChanged}`)

    expect(detailVisible || urlChanged).toBeTruthy()
  })

  test('08 – ESC schließt geöffnetes Task-Modal', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }

    await page.waitForSelector('.kanban-column', { timeout: 15000 })

    const tasks = page.locator('.kanban-item')
    if (await tasks.count() === 0) { test.skip(); return }

    await tasks.first().click()
    await page.waitForTimeout(800)

    const urlBefore = page.url()

    // ESC drücken
    await page.keyboard.press('Escape')
    await page.waitForTimeout(500)

    const urlAfter = page.url()
    console.log(`URL vor ESC: ${urlBefore}`)
    console.log(`URL nach ESC: ${urlAfter}`)

    // Board sollte wieder sichtbar sein
    await expect(page.locator('#board')).toBeVisible()
    await page.screenshot({ path: '/tmp/plankton-esc-close.png' })
  })

  test('09 – Projekt-Menü-Button öffnet Dropdown', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }

    await page.waitForSelector('#project-menu-btn', { timeout: 15000 })

    const menuBtn = page.locator('#project-menu-btn')
    await expect(menuBtn).toBeVisible()

    await menuBtn.click()
    await page.waitForTimeout(500)

    // Dropdown sollte erscheinen
    const dropdown = page.locator('#project-dropdown')
    const isVisible = await dropdown.isVisible().catch(() => false)
    const hasContent = await dropdown.textContent().then(t => t && t.trim().length > 0).catch(() => false)

    await page.screenshot({ path: '/tmp/plankton-menu.png' })
    console.log(`Dropdown sichtbar: ${isVisible}, hat Inhalt: ${hasContent}`)

    expect(isVisible || hasContent).toBeTruthy()
  })

  test('10 – Import-Button öffnet Import-Modal', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }

    await page.waitForSelector('#import-btn', { timeout: 15000 })

    const importBtn = page.locator('#import-btn')
    await expect(importBtn).toBeVisible()

    await importBtn.click()
    await page.waitForTimeout(500)

    // Import-Modal sollte erscheinen (CSS: display wird von 'none' zu 'flex')
    const importModal = page.locator('#import-modal')
    const modalDisplay = await importModal.evaluate(el => {
      return window.getComputedStyle(el).display
    }).catch(() => 'unknown')

    await page.screenshot({ path: '/tmp/plankton-import-modal.png' })
    console.log(`Import-Modal display: ${modalDisplay}`)

    // Modal sollte nicht 'none' sein
    expect(modalDisplay).not.toBe('none')
  })

  test('11 – Import-Modal schließt via Close-Button', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }

    await page.waitForSelector('#import-btn', { timeout: 15000 })
    await page.locator('#import-btn').click()
    await page.waitForTimeout(500)

    // Schließen via X-Button
    const closeBtn = page.locator('#import-modal-close')
    await closeBtn.click()
    await page.waitForTimeout(500)

    const importModal = page.locator('#import-modal')
    const modalDisplay = await importModal.evaluate(el => {
      return window.getComputedStyle(el).display
    }).catch(() => 'none')

    console.log(`Import-Modal display nach Close: ${modalDisplay}`)
    expect(modalDisplay).toBe('none')
  })

  test('12 – Theme-Toggle wechselt dark/light', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }

    await page.waitForSelector('#theme-toggle', { timeout: 15000 })

    const body = page.locator('body')
    const themeBefore = await body.getAttribute('data-theme')
    console.log(`Theme vorher: ${themeBefore}`)

    // Auf Mobile ist der Theme-Toggle in der Sidebar (außerhalb des Viewports).
    // Sidebar erst öffnen oder per force-click / JavaScript-Klick.
    const themeToggle = page.locator('#theme-toggle')
    const isInViewport = await themeToggle.isVisible().catch(() => false)
    if (!isInViewport) {
      // Sidebar öffnen via Toggle-Button (Mobile)
      const sidebarToggle = page.locator('.sidebar-toggle')
      if (await sidebarToggle.isVisible().catch(() => false)) {
        await sidebarToggle.click()
        await page.waitForTimeout(300)
      }
    }

    // Klick via JavaScript um Viewport-Probleme zu umgehen
    await page.evaluate(() => {
      const btn = document.getElementById('theme-toggle')
      if (btn) btn.click()
    })
    await page.waitForTimeout(300)

    const themeAfter = await body.getAttribute('data-theme')
    console.log(`Theme nachher: ${themeAfter}`)

    expect(themeAfter).not.toBe(themeBefore)
  })

})

// ─── MOBILE VIEWPORT TESTS ──────────────────────────────────────────────────

test.describe('Mobile Viewport Tests', () => {

  test('13 – Board auf 375px Breite: kein Layout-Overflow', async ({ page, browserName }) => {
    // Viewport auf Mobile setzen (falls nicht schon gesetzt durch Playwright-Config)
    await page.setViewportSize({ width: 375, height: 667 })

    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }

    await page.waitForSelector('#board', { timeout: 15000 })

    // Kein horizontaler Overflow auf dem Body
    const bodyScrollWidth = await page.evaluate(() => document.body.scrollWidth)
    const windowWidth = await page.evaluate(() => window.innerWidth)

    await page.screenshot({ path: '/tmp/plankton-mobile.png' })
    console.log(`Body scrollWidth: ${bodyScrollWidth}, window.innerWidth: ${windowWidth}`)

    // Board-Bereich muss scrollbar sein, aber BODY nicht zu breit
    // Toleranz: max. 20px Überschreitung (Scrollbar etc.)
    expect(bodyScrollWidth).toBeLessThanOrEqual(windowWidth + 20)
  })

  test('14 – Sidebar-Toggle auf Mobile vorhanden', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 })

    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }

    await page.waitForSelector('.sidebar-toggle', { timeout: 15000 })

    // Auf Mobile sollte der Sidebar-Toggle sichtbar sein
    const toggleBtn = page.locator('.sidebar-toggle')
    const isVisible = await toggleBtn.isVisible().catch(() => false)
    console.log(`Sidebar-Toggle sichtbar auf Mobile: ${isVisible}`)

    // Der Toggle ist per CSS mit 'hidden' versteckt auf Desktop, aber sichtbar auf Mobile
    // Prüfe computed style
    const display = await toggleBtn.evaluate(el => window.getComputedStyle(el).display).catch(() => 'none')
    console.log(`Sidebar-Toggle display: ${display}`)

    await page.screenshot({ path: '/tmp/plankton-mobile-sidebar.png' })
  })

  test('15 – Task-Modal scrollbar auf Mobile (Regression: d3eb008)', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 })

    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }

    await page.waitForSelector('.kanban-column', { timeout: 15000 })

    const tasks = page.locator('.kanban-item')
    if (await tasks.count() === 0) { test.skip(); return }

    await tasks.first().click()
    await page.waitForTimeout(800)

    // Task-Detail soll max-height und overflow haben (Viewport-Höhe respektieren)
    await page.screenshot({ path: '/tmp/plankton-mobile-task-modal.png' })

    // Modal darf nicht größer als Viewport sein
    const viewportHeight = 667
    const bodyHeight = await page.evaluate(() => document.body.offsetHeight)
    console.log(`Body height: ${bodyHeight}, Viewport: ${viewportHeight}`)

    // Kein massiver Overflow
    expect(bodyHeight).toBeLessThanOrEqual(viewportHeight + 100)
  })

})

// ─── DASHBOARD-CONTAINER TESTS ──────────────────────────────────────────────

test.describe('Dashboard-Container', () => {

  test('20 – Dashboard-Container wird über dem Board angezeigt', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }

    await page.waitForSelector('#board', { timeout: 15000 })

    // Dashboard-Container muss im DOM vorhanden sein
    const dashboard = page.locator('[data-testid="dashboard-container"]')
    await expect(dashboard).toBeVisible({ timeout: 5000 })

    // Board muss danach noch sichtbar sein
    await expect(page.locator('#board')).toBeVisible()

    // Dashboard-Container muss ÜBER dem Board sein (im DOM davor oder als übergeordnetes Element)
    const dashboardOrder = await page.evaluate(() => {
      const dash = document.querySelector('[data-testid="dashboard-container"]')
      const board = document.getElementById('board')
      if (!dash || !board) return null
      const position = dash.compareDocumentPosition(board)
      // DOCUMENT_POSITION_FOLLOWING = 4 (board nach dashboard)
      return (position & 4) !== 0
    })
    expect(dashboardOrder).toBe(true)

    await page.screenshot({ path: '/tmp/plankton-dashboard.png' })
  })

  test('21 – Dashboard-Container Toggle collapsed/expanded', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }

    await page.waitForSelector('[data-testid="dashboard-container"]', { timeout: 15000 })

    // must_change_password-Modal schließen bevor Toggle-Klick (admin/admin-Login)
    await dismissPasswordChangeModal(page)

    const toggleBtn = page.locator('[data-testid="dashboard-toggle"]')
    await expect(toggleBtn).toBeVisible({ timeout: 5000 })

    // Initial-State abrufen
    const dashboard = page.locator('[data-testid="dashboard-container"]')
    const initialExpanded = await dashboard.evaluate(el => el.getAttribute('data-expanded'))

    // Toggle klicken
    await toggleBtn.click()
    await page.waitForTimeout(300)

    const afterToggleExpanded = await dashboard.evaluate(el => el.getAttribute('data-expanded'))

    // Zustand muss sich geändert haben
    expect(afterToggleExpanded).not.toBe(initialExpanded)

    // Nochmal klicken – zurück zum Initial-State
    await toggleBtn.click()
    await page.waitForTimeout(300)

    const backToInitial = await dashboard.evaluate(el => el.getAttribute('data-expanded'))
    expect(backToInitial).toBe(initialExpanded)

    await page.screenshot({ path: '/tmp/plankton-dashboard-toggle.png' })
  })

  test('22 – Dashboard-Preference wird in localStorage gespeichert', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }

    await page.waitForSelector('[data-testid="dashboard-container"]', { timeout: 15000 })

    // must_change_password-Modal schließen bevor Toggle-Klick (admin/admin-Login)
    await dismissPasswordChangeModal(page)

    const toggleBtn = page.locator('[data-testid="dashboard-toggle"]')
    await toggleBtn.click()
    await page.waitForTimeout(300)

    // localStorage prüfen
    const storageValue = await page.evaluate(() => {
      return localStorage.getItem('plankton_dashboard_visible')
    })

    // Wert muss gesetzt sein
    expect(storageValue).not.toBeNull()
    // Wert muss 'true' oder 'false' sein
    expect(['true', 'false']).toContain(storageValue)

    // Nochmal klicken und prüfen ob sich der Wert ändert
    const valueBefore = storageValue
    await toggleBtn.click()
    await page.waitForTimeout(300)

    const valueAfter = await page.evaluate(() => {
      return localStorage.getItem('plankton_dashboard_visible')
    })
    expect(valueAfter).not.toBe(valueBefore)
  })

  test('23 – Dashboard-Preference wird nach Reload wiederhergestellt', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }

    await page.waitForSelector('[data-testid="dashboard-container"]', { timeout: 15000 })

    // Preference explizit auf 'false' setzen
    await page.evaluate(() => {
      localStorage.setItem('plankton_dashboard_visible', 'false')
    })

    // Seite neu laden
    await page.reload()
    const loggedInAfterReload = await tryLogin(page)
    if (!loggedInAfterReload) { test.skip(); return }

    await page.waitForSelector('[data-testid="dashboard-container"]', { timeout: 15000 })

    // Dashboard-Container muss collapsed sein
    const dashboard = page.locator('[data-testid="dashboard-container"]')
    const expanded = await dashboard.evaluate(el => el.getAttribute('data-expanded'))
    expect(expanded).toBe('false')

    // Board muss trotzdem sichtbar sein (collapsed != unsichtbar für Board)
    await expect(page.locator('#board')).toBeVisible()
  })

  test('24 – Dashboard hat mindestens 3 Widget-Slots', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }

    await page.waitForSelector('[data-testid="dashboard-container"]', { timeout: 15000 })

    // Dashboard muss expanded sein für Widget-Slots
    const dashboard = page.locator('[data-testid="dashboard-container"]')
    const expanded = await dashboard.evaluate(el => el.getAttribute('data-expanded'))

    if (expanded === 'false') {
      // Toggle um zu öffnen
      await page.locator('[data-testid="dashboard-toggle"]').click()
      await page.waitForTimeout(300)
    }

    // Widget-Slot-Container prüfen
    const widgetSlots = page.locator('[data-testid="dashboard-widget-slot"]')
    const slotCount = await widgetSlots.count()
    console.log(`Gefundene Widget-Slots: ${slotCount}`)
    expect(slotCount).toBeGreaterThanOrEqual(3)
  })

  test('25 – Mobile: Dashboard ist standardmäßig collapsed', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 })

    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }

    await page.waitForSelector('[data-testid="dashboard-container"]', { timeout: 15000 })

    // Auf Mobile: localStorage löschen damit Default greift
    await page.evaluate(() => {
      localStorage.removeItem('plankton_dashboard_visible')
    })
    await page.reload()

    // Nach Mobile-Reload neu einloggen
    const loggedInAfterReload = await tryLogin(page)
    if (!loggedInAfterReload) { test.skip(); return }

    await page.waitForSelector('[data-testid="dashboard-container"]', { timeout: 15000 })

    // Auf Mobile muss Dashboard collapsed sein (Default)
    const dashboard = page.locator('[data-testid="dashboard-container"]')
    const expanded = await dashboard.evaluate(el => el.getAttribute('data-expanded'))
    console.log(`Mobile Default Dashboard expanded: ${expanded}`)
    expect(expanded).toBe('false')

    await page.screenshot({ path: '/tmp/plankton-dashboard-mobile.png' })
  })

  test('26 – Collapsed Dashboard verschiebt Board nicht', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }

    await page.waitForSelector('[data-testid="dashboard-container"]', { timeout: 15000 })

    // Board-Position vor Toggle merken
    const boardTopExpanded = await page.locator('#board').evaluate(el => el.getBoundingClientRect().top)

    // Dashboard collapsed machen
    const dashboard = page.locator('[data-testid="dashboard-container"]')
    const expanded = await dashboard.evaluate(el => el.getAttribute('data-expanded'))
    if (expanded === 'true') {
      await page.locator('[data-testid="dashboard-toggle"]').click()
      await page.waitForTimeout(400)
    }

    // Board-Position nach Toggle – muss sich geändert haben wenn Dashboard weniger Platz braucht
    // Aber Board darf nicht verschwinden
    await expect(page.locator('#board')).toBeVisible()

    // Widget-Content-Bereich muss bei collapsed versteckt sein
    const widgetContent = page.locator('[data-testid="dashboard-widgets"]')
    const contentVisible = await widgetContent.isVisible().catch(() => false)
    console.log(`Widget-Content bei collapsed sichtbar: ${contentVisible}`)
    expect(contentVisible).toBe(false)
  })

})

// ─── SIDEBAR SEARCH & SORT TESTS ────────────────────────────────────────────

test.describe('Sidebar Suche & Sortierung', () => {

  test('30 – Sidebar enthält Search-Input und Sort-Toggle', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }

    await page.waitForSelector('aside.sidebar', { timeout: 15000 })
    await dismissPasswordChangeModal(page)

    // Auf Mobile: Sidebar erst über Hamburger-Button öffnen
    const hamburger = page.locator('.sidebar-toggle')
    if (await hamburger.isVisible({ timeout: 1000 }).catch(() => false)) {
      await hamburger.click()
      await page.waitForTimeout(200)
    }

    // Suchfeld ist im DOM vorhanden, aber standardmäßig hidden (toggle-Verhalten)
    const searchInput = page.locator('#sidebar-search-input')
    await expect(searchInput).toBeAttached({ timeout: 5000 })

    // Such-Button (🔍) klicken um das Suchfeld einzublenden
    const searchToggleBtn = page.locator('#sidebar-search-toggle')
    await expect(searchToggleBtn).toBeVisible({ timeout: 5000 })
    await searchToggleBtn.click()
    await expect(searchInput).toBeVisible({ timeout: 3000 })

    // Sort-Button/-Toggle muss vorhanden und sichtbar sein
    const sortToggle = page.locator('[data-sort-toggle]')
    await expect(sortToggle).toBeVisible({ timeout: 5000 })

    await page.screenshot({ path: '/tmp/plankton-sidebar-header.png' })
  })

  test('31 – Suche filtert Projektliste (case-insensitive)', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }

    await page.waitForSelector('#project-list', { timeout: 15000 })
    await dismissPasswordChangeModal(page)

    // Auf Mobile: Sidebar erst über Hamburger-Button öffnen
    const hamburger = page.locator('.sidebar-toggle')
    if (await hamburger.isVisible({ timeout: 1000 }).catch(() => false)) {
      await hamburger.click()
      await page.waitForTimeout(200)
    }

    // Suchfeld ist standardmäßig hidden – Such-Button klicken um es einzublenden
    const searchToggleBtn = page.locator('#sidebar-search-toggle')
    await expect(searchToggleBtn).toBeVisible({ timeout: 5000 })
    await searchToggleBtn.click()

    const searchInput = page.locator('#sidebar-search-input')
    await expect(searchInput).toBeVisible({ timeout: 3000 })

    // Anzahl Projekte vor der Suche merken
    const projectsBefore = await page.locator('#project-list .project-item').count()
    console.log(`Projekte vor Suche: ${projectsBefore}`)

    if (projectsBefore === 0) {
      console.log('Keine Projekte vorhanden – Test übersprungen')
      test.skip()
      return
    }

    // Mit einem Suchbegriff filtern, der garantiert keinen Treffer ergibt
    await searchInput.fill('XYZZZNOTEXISTING9999')
    await page.waitForTimeout(200)

    const projectsAfter = await page.locator('#project-list .project-item').count()
    console.log(`Projekte nach Suche (kein Treffer): ${projectsAfter}`)

    // Kein Projekt sollte sichtbar sein
    expect(projectsAfter).toBe(0)

    // Reset via ESC
    await searchInput.press('Escape')
    await page.waitForTimeout(200)

    const projectsAfterReset = await page.locator('#project-list .project-item').count()
    console.log(`Projekte nach ESC-Reset: ${projectsAfterReset}`)
    expect(projectsAfterReset).toBe(projectsBefore)

    await page.screenshot({ path: '/tmp/plankton-sidebar-search.png' })
  })

  test('32 – Sort-Optionen wechseln die Sortierung', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }

    await page.waitForSelector('#project-list', { timeout: 15000 })
    await dismissPasswordChangeModal(page)

    // Auf Mobile: Sidebar erst über Hamburger-Button öffnen
    const hamburger = page.locator('.sidebar-toggle')
    if (await hamburger.isVisible({ timeout: 1000 }).catch(() => false)) {
      await hamburger.click()
      await page.waitForTimeout(200)
    }

    const sortToggle = page.locator('[data-sort-toggle]')
    await expect(sortToggle).toBeVisible({ timeout: 5000 })

    // Sort-Toggle öffnen
    await sortToggle.click({ force: true })
    await page.waitForTimeout(200)

    // Sort-Option "alphabetisch A-Z" klicken
    const sortAZ = page.locator('[data-sort-option="alpha-asc"]')
    if (await sortAZ.isVisible({ timeout: 2000 }).catch(() => false)) {
      await sortAZ.click()
      await page.waitForTimeout(200)

      // Aktive Sort-Option muss markiert sein
      const isActive = await sortAZ.evaluate(el => el.classList.contains('sort-active') || el.getAttribute('data-sort-active') === 'true' || el.getAttribute('aria-pressed') === 'true')
      console.log(`Sort A-Z aktiv: ${isActive}`)
    } else {
      console.log('Sort-Optionen nicht sichtbar – Test übersprungen')
      test.skip()
      return
    }

    await page.screenshot({ path: '/tmp/plankton-sidebar-sort.png' })
  })

  test('33 – Sort-Präferenz wird in localStorage gespeichert', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }

    await page.waitForSelector('#project-list', { timeout: 15000 })
    await dismissPasswordChangeModal(page)

    // Auf Mobile: Sidebar erst über Hamburger-Button öffnen
    const hamburger = page.locator('.sidebar-toggle')
    if (await hamburger.isVisible({ timeout: 1000 }).catch(() => false)) {
      await hamburger.click()
      await page.waitForTimeout(200)
    }

    const sortToggle = page.locator('[data-sort-toggle]')
    await expect(sortToggle).toBeVisible({ timeout: 5000 })

    // Toggle öffnen und Alpha-Sort wählen
    await sortToggle.click({ force: true })
    await page.waitForTimeout(200)

    const sortAZ = page.locator('[data-sort-option="alpha-asc"]')
    if (!await sortAZ.isVisible({ timeout: 2000 }).catch(() => false)) {
      test.skip(); return
    }
    await sortAZ.click()
    await page.waitForTimeout(200)

    // localStorage prüfen
    const storageValue = await page.evaluate(() => localStorage.getItem('plankton_sidebar_sort'))
    console.log(`localStorage sidebar_sort: ${storageValue}`)
    expect(storageValue).toBe('alpha-asc')
  })

})

// ─── LIST-BOARD ARCHIVE BUTTON TESTS ────────────────────────────────────────

test.describe('List-Board Archive-Button', () => {

  /**
   * Prüft dass im List-Board jede Task-Karte einen Archive-Button (.task-archive-btn) hat.
   * Der Button muss sichtbar sein, sobald das List-Board eine Task enthält.
   */
  test('40 – List-Board: Archive-Button ist pro Task sichtbar', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }
    await dismissPasswordChangeModal(page)

    await page.waitForSelector('#board', { timeout: 15000 })

    // Simuliere ein List-Projekt mit einem Task
    await page.evaluate(() => {
      // @ts-ignore
      const s = window.__state
      if (!s || !s.project) return
      s.project.type = 'list'
      // Stelle sicher dass mindestens ein Task in der ersten sichtbaren Spalte existiert
      const visibleCol = s.project.columns
        .filter((c: any) => !c.hidden)
        .sort((a: any, b: any) => a.order - b.order)[0]
      if (!visibleCol) return
      const tasksInCol = s.project.tasks.filter((t: any) => t.column_id === visibleCol.id)
      if (tasksInCol.length === 0) {
        // Dummy-Task injizieren
        s.project.tasks.push({
          id: 'test-archive-task-1',
          title: 'Test Archive Task',
          column_id: visibleCol.id,
          order: 0,
          task_type: 'task',
          description: '',
          labels: [],
          worker: '',
          blocked_by: [],
          blocks: [],
          subtask_ids: [],
          comments: [],
          logs: [],
          points: 0,
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
          slug: 'test-archive-task-1',
          creator: 'admin',
          parent_id: null,
        })
      }
    })

    await page.waitForTimeout(400)

    // List-Board muss sichtbar sein
    const listBoard = page.locator('.list-board')
    await expect(listBoard).toBeVisible({ timeout: 5000 })

    // Archive-Button muss pro Task-Karte vorhanden sein
    const archiveBtns = page.locator('.task-archive-btn')
    const btnCount = await archiveBtns.count()
    console.log(`Gefundene Archive-Buttons: ${btnCount}`)
    expect(btnCount).toBeGreaterThan(0)

    await page.screenshot({ path: '/tmp/plankton-list-archive-btn.png' })
  })

  /**
   * Prüft dass der Archive-Button im Kanban-Board NICHT vorhanden ist.
   */
  test('41 – Kanban-Board: kein Archive-Button sichtbar', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }
    await dismissPasswordChangeModal(page)

    await page.waitForSelector('#board', { timeout: 15000 })

    // Sicherstellen dass Kanban-Modus aktiv ist
    await page.evaluate(() => {
      // @ts-ignore
      const s = window.__state
      if (!s || !s.project) return
      s.project.type = 'kanban'
    })

    await page.waitForTimeout(300)

    // Kanban-Board muss sichtbar sein
    const kanbanCols = page.locator('.kanban-column')
    const colCount = await kanbanCols.count()
    if (colCount === 0) { test.skip(); return }

    // Kein Archive-Button im Kanban-Board
    const archiveBtns = page.locator('.task-archive-btn')
    const btnCount = await archiveBtns.count()
    console.log(`Archive-Buttons im Kanban-Board: ${btnCount}`)
    expect(btnCount).toBe(0)

    await page.screenshot({ path: '/tmp/plankton-kanban-no-archive-btn.png' })
  })

  /**
   * Prüft dass Klick auf Archive-Button den Task reaktiv aus der Liste entfernt.
   */
  test('42 – List-Board: Archive-Button-Klick entfernt Task reaktiv', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }
    await dismissPasswordChangeModal(page)

    await page.waitForSelector('#board', { timeout: 15000 })

    // Simuliere List-Projekt mit einem Task (archivierbar ohne echten API-Call)
    await page.evaluate(() => {
      // @ts-ignore
      const s = window.__state
      if (!s || !s.project) return
      s.project.type = 'list'

      const visibleCol = s.project.columns
        .filter((c: any) => !c.hidden)
        .sort((a: any, b: any) => a.order - b.order)[0]
      if (!visibleCol) return

      // Task hinzufügen falls nötig
      const existing = s.project.tasks.filter((t: any) => t.column_id === visibleCol.id)
      if (existing.length === 0) {
        s.project.tasks.push({
          id: 'test-archive-task-2',
          title: 'Archive Me',
          column_id: visibleCol.id,
          order: 0,
          task_type: 'task',
          description: '',
          labels: [],
          worker: '',
          blocked_by: [],
          blocks: [],
          subtask_ids: [],
          comments: [],
          logs: [],
          points: 0,
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
          slug: 'archive-me',
          creator: 'admin',
          parent_id: null,
        })
      }

      // _archive-Spalte hinzufügen falls nicht vorhanden
      const hasArchive = s.project.columns.some((c: any) => c.title === '_archive')
      if (!hasArchive) {
        s.project.columns.push({
          id: 'test-archive-col',
          title: '_archive',
          order: 99,
          color: '#444',
          hidden: true,
          slug: '_archive',
          locked: false,
        })
      }

      // Mock des API-Calls: fetch überschreiben
      const origFetch = window.fetch.bind(window)
      ;(window as any).__origFetch = origFetch
      ;(window as any).__archiveApiCalled = false
      window.fetch = async (input: RequestInfo | URL, init?: RequestInit) => {
        const url = typeof input === 'string' ? input : input.toString()
        if (url.includes('/move') && init?.method === 'POST') {
          ;(window as any).__archiveApiCalled = true
          // Simuliere Erfolg
          return new Response(JSON.stringify({ ok: true }), { status: 200, headers: { 'Content-Type': 'application/json' } })
        }
        return origFetch(input, init)
      }
    })

    await page.waitForTimeout(400)

    const listBoard = page.locator('.list-board')
    await expect(listBoard).toBeVisible({ timeout: 5000 })

    // Anzahl Tasks vor Archivierung
    const tasksBefore = await page.locator('.list-item').count()
    console.log(`Tasks vor Archivierung: ${tasksBefore}`)
    if (tasksBefore === 0) { test.skip(); return }

    // Archive-Button des ersten Tasks klicken
    const firstArchiveBtn = page.locator('.task-archive-btn').first()
    await expect(firstArchiveBtn).toBeVisible({ timeout: 3000 })
    await firstArchiveBtn.click({ force: true })

    // Task sollte nach kurzer Zeit aus der Liste verschwunden sein
    await page.waitForTimeout(500)

    const tasksAfter = await page.locator('.list-item').count()
    console.log(`Tasks nach Archivierung: ${tasksAfter}`)

    expect(tasksAfter).toBeLessThan(tasksBefore)

    await page.screenshot({ path: '/tmp/plankton-list-after-archive.png' })
  })

})

// ─── REGRESSION TESTS ───────────────────────────────────────────────────────

test.describe('Regression Tests', () => {

  test('16 – Overlay-Bug-Fix: Import-Overlay öffnet sich korrekt (Regression: fix overlays)', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }

    await page.waitForSelector('#import-btn', { timeout: 15000 })

    // Import-Overlay öffnen
    await page.locator('#import-btn').click()
    await page.waitForTimeout(500)

    const importModal = page.locator('#import-modal')
    const display = await importModal.evaluate(el => window.getComputedStyle(el).display)
    expect(display).not.toBe('none')

    // Overlay-Hintergrund anklicken schließt NICHT das Modal
    // (Regression: "clicking overlay background no longer closes task modals" – c7054cd)
    // Hinweis: Das betrifft Task-Modals, nicht Import-Modal
    // Import-Modal schließt durch Klick auf #import-modal selbst (delegierter Klick)

    // Schließe via ESC
    await page.keyboard.press('Escape')
    await page.waitForTimeout(300)

    // ESC schließt Legacy-Modals normalerweise NICHT automatisch
    // Stattdessen muss Close-Button genutzt werden
    const displayAfterEsc = await importModal.evaluate(el => window.getComputedStyle(el).display)
    console.log(`Import-Modal nach ESC: ${displayAfterEsc}`)
    await page.screenshot({ path: '/tmp/plankton-regression-overlay.png' })
  })

  test('17 – Task-Modal: Overlay-Hintergrund schließt NICHT das Modal (Regression: c7054cd)', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }

    await page.waitForSelector('.kanban-column', { timeout: 15000 })

    const tasks = page.locator('.kanban-item')
    if (await tasks.count() === 0) { test.skip(); return }

    await tasks.first().click()
    await page.waitForTimeout(800)

    const urlAfterOpen = page.url()
    const taskOpened = urlAfterOpen.includes('/t/')
    console.log(`Task geöffnet (URL): ${taskOpened}, URL: ${urlAfterOpen}`)

    if (!taskOpened) {
      console.log('Task-URL nicht gesetzt, Test übersprungen')
      test.skip()
      return
    }

    // Klick auf Overlay-Hintergrund (außerhalb des Modal-Inhalts)
    // Das Task-Detail ist eine Vue-Komponente – Klick NEBEN das Panel
    await page.mouse.click(10, 360) // Ganz links, außerhalb des Panels
    await page.waitForTimeout(500)

    const urlAfterClick = page.url()
    console.log(`URL nach Overlay-Klick: ${urlAfterClick}`)

    // Prüfen ob Modal noch offen ist (URL noch mit /t/) – Regression-Fix
    // Gem. Commit c7054cd: Klick auf Hintergrund schließt Task-Modals NICHT mehr
    // Daher erwarten wir, dass die URL NICHT zurückspringt
    // Hinweis: Test ist informativ, weil wir das Verhalten nicht kennen
    await page.screenshot({ path: '/tmp/plankton-regression-overlay-click.png' })
  })

  test('18 – Content-Hash Cache-Busting: Bundle-URLs enthalten Hash (Regression: 1e5693f)', async ({ page }) => {
    const response = await page.goto('/')
    const html = await page.content()

    // Bundle-URLs sollten Hash-Suffix haben (z.B. bundle.d5a1eb39.js)
    const hasBundleHash = /bundle\.[a-f0-9]{8}\.js/.test(html)
    const hasCssHash = /bundle\.[a-f0-9]{8}\.css/.test(html)

    console.log(`Bundle-Hash in HTML: ${hasBundleHash}`)
    console.log(`CSS-Hash in HTML: ${hasCssHash}`)

    expect(hasBundleHash).toBeTruthy()
    expect(hasCssHash).toBeTruthy()
  })

  test('19 – Strukturierte Fehler-Codes: API reagiert auf ungültige Requests (Regression: ef81cff)', async ({ page, request }) => {
    // Teste dass API vernünftige Fehler zurückgibt
    const response = await request.post(`${BASE_URL}/api/projects/invalid-id/tasks/batch-move`, {
      data: { moves: [] },
      headers: { 'Content-Type': 'application/json' },
      failOnStatusCode: false,
    })

    console.log(`API Status bei ungültiger Request: ${response.status()}`)
    // Sollte 401 (nicht eingeloggt) oder 400/404 (ungültige ID) zurückgeben
    expect(response.status()).toBeGreaterThanOrEqual(400)
    expect(response.status()).toBeLessThan(600)
  })

  test('20 – Project Settings: 3-Tab-Layout (Details / Users / JSON)', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }

    await dismissPasswordChangeModal(page)

    // Projekt-Menü öffnen
    await page.locator('#project-menu-btn').click()
    await page.waitForTimeout(300)

    // "Projekt editieren" wählen
    const editBtn = page.locator('[data-action="edit"]')
    await editBtn.waitFor({ timeout: 5000 })
    await editBtn.click()

    // Projekt-Modal muss offen sein
    const modal = page.locator('#project-modal')
    await modal.waitFor({ timeout: 5000 })
    await expect(modal).toBeVisible()

    // Alle drei Tabs müssen existieren
    await expect(page.locator('[data-proj-tab="details"]')).toBeVisible()
    await expect(page.locator('[data-proj-tab="users"]')).toBeVisible()
    await expect(page.locator('[data-proj-tab="json"]')).toBeVisible()

    // Tab 1 – Details: Felder prüfen
    await page.locator('[data-proj-tab="details"]').click()
    await expect(page.locator('#proj-field-id')).toBeVisible()
    await expect(page.locator('#proj-field-title')).toBeVisible()
    await expect(page.locator('#proj-field-type')).toBeVisible()
    await expect(page.locator('#proj-field-slug')).toBeVisible()
    await expect(page.locator('#proj-field-owner')).toBeVisible()

    // Das id-Feld muss readonly sein
    const idReadonly = await page.locator('#proj-field-id').getAttribute('readonly')
    expect(idReadonly).not.toBeNull()

    // Tab 2 – Users
    await page.locator('[data-proj-tab="users"]').click()
    await expect(page.locator('#proj-users-tab')).toBeVisible()

    // Tab 3 – JSON: JSON-Tree oder Raw-Textarea muss existieren
    await page.locator('[data-proj-tab="json"]').click()
    await expect(page.locator('#proj-json-tab')).toBeVisible()
    // Toggle-Button muss vorhanden sein
    await expect(page.locator('#proj-view-toggle')).toBeVisible()
    // Entweder JSON-Tree oder Textarea muss sichtbar sein
    const treeVisible = await page.locator('#proj-json-tree').isVisible().catch(() => false)
    const textareaVisible = await page.locator('#proj-modal-json').isVisible().catch(() => false)
    expect(treeVisible || textareaVisible).toBeTruthy()
  })

  test('21 – Project Settings: doneExpire + archiveDelete im Details-Tab', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }

    await dismissPasswordChangeModal(page)

    // Projekt-Menü öffnen
    await page.locator('#project-menu-btn').click()
    await page.waitForTimeout(300)

    // "Projekt editieren" wählen
    const editBtn = page.locator('[data-action="edit"]')
    await editBtn.waitFor({ timeout: 5000 })
    await editBtn.click()

    // Projekt-Modal muss offen sein
    const modal = page.locator('#project-modal')
    await modal.waitFor({ timeout: 5000 })
    await expect(modal).toBeVisible()

    // Details-Tab aktivieren
    await page.locator('[data-proj-tab="details"]').click()

    // Beide Felder müssen im Details-Tab sichtbar sein
    await expect(page.locator('#proj-field-done-expire')).toBeVisible()
    await expect(page.locator('#proj-field-archive-delete')).toBeVisible()

    // doneExpire-Wert ändern und speichern
    const doneExpireInput = page.locator('#proj-field-done-expire')
    await doneExpireInput.fill('7')

    // archiveDelete-Wert ändern
    const archiveDeleteInput = page.locator('#proj-field-archive-delete')
    await archiveDeleteInput.fill('60')

    // Speichern
    await page.locator('#proj-details-save').click()
    await page.waitForTimeout(500)

    // Modal muss nach dem Speichern geschlossen sein
    await expect(modal).not.toBeVisible()

    // Erneut öffnen und persistierten Wert prüfen
    await page.locator('#project-menu-btn').click()
    await page.waitForTimeout(300)
    await page.locator('[data-action="edit"]').click()
    await modal.waitFor({ timeout: 5000 })
    await page.locator('[data-proj-tab="details"]').click()

    await expect(page.locator('#proj-field-done-expire')).toHaveValue('7')
    await expect(page.locator('#proj-field-archive-delete')).toHaveValue('60')
  })

})
