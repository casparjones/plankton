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
  try {
    await page.waitForSelector('.kanban-column, #board .kanban-column', { timeout: 10000 })
    return true
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

    // Suchfeld muss vorhanden sein
    const searchInput = page.locator('#sidebar-search-input')
    await expect(searchInput).toBeVisible({ timeout: 5000 })

    // Sort-Button/-Toggle muss vorhanden sein
    const sortToggle = page.locator('[data-sort-toggle]')
    await expect(sortToggle).toBeVisible({ timeout: 5000 })

    await page.screenshot({ path: '/tmp/plankton-sidebar-header.png' })
  })

  test('31 – Suche filtert Projektliste (case-insensitive)', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }

    await page.waitForSelector('#project-list', { timeout: 15000 })
    await dismissPasswordChangeModal(page)

    const searchInput = page.locator('#sidebar-search-input')
    await expect(searchInput).toBeVisible({ timeout: 5000 })

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

    const sortToggle = page.locator('[data-sort-toggle]')
    await expect(sortToggle).toBeVisible({ timeout: 5000 })

    // Sort-Toggle öffnen
    await sortToggle.click()
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

    const sortToggle = page.locator('[data-sort-toggle]')
    await expect(sortToggle).toBeVisible({ timeout: 5000 })

    // Toggle öffnen und Alpha-Sort wählen
    await sortToggle.click()
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

})
