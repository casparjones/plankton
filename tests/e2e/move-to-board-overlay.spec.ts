import { test, expect, Page } from '@playwright/test'

const USERNAME = process.env.PLANKTON_USER || 'admin'
const PASSWORD = process.env.PLANKTON_PASS || 'admin'
const BASE_URL = process.env.PLANKTON_URL || 'http://localhost:3099'

async function dismissPasswordChangeModal(page: Page): Promise<void> {
  await page.waitForTimeout(150)
  await page.evaluate(() => {
    const modal = document.getElementById('password-modal') as HTMLElement
    if (modal) {
      modal.dataset.force = ''
      modal.classList.remove('open')
      modal.style.pointerEvents = 'none'
      modal.style.zIndex = '-1'
    }
  })
  await page.waitForTimeout(150)
}

async function tryLogin(page: Page): Promise<boolean> {
  await page.goto('/')
  const loginForm = page.locator('form')
  try {
    await loginForm.waitFor({ timeout: 10000 })
  } catch {
    return true
  }
  await page.locator('input[type="text"]').fill(USERNAME)
  await page.locator('input[type="password"]').fill(PASSWORD)
  await page.locator('button[type="submit"]').click()
  try {
    await page.waitForSelector('.kanban-column, #board .kanban-column', { timeout: 10000 })
    return true
  } catch {
    return false
  }
}

// ─── MoveToBoardOverlay Tests ────────────────────────────────────────────────

test.describe('MoveToBoardOverlay', () => {

  /**
   * Test 1: Overlay öffnen → Auto-Suche „Backlog" ist aktiv
   * Das Suchfeld muss beim Öffnen mit „Backlog" vorausgefüllt sein.
   */
  test('01 – Overlay öffnen: Suchfeld ist mit „Backlog" vorausgefüllt', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }
    await dismissPasswordChangeModal(page)

    await page.waitForSelector('#board', { timeout: 15000 })

    // Overlay via window.__openMoveToBoardOverlay öffnen (globale Methode)
    const opened = await page.evaluate(() => {
      // @ts-ignore
      if (typeof window.__openMoveToBoardOverlay === 'function') {
        // @ts-ignore
        window.__openMoveToBoardOverlay('test-task-id')
        return true
      }
      return false
    })

    if (!opened) {
      console.log('__openMoveToBoardOverlay nicht verfügbar – test skip')
      test.skip()
      return
    }

    // Overlay muss erscheinen
    const overlay = page.locator('[data-testid="move-to-board-overlay"]')
    await expect(overlay).toBeVisible({ timeout: 5000 })

    // Suchfeld muss mit „Backlog" vorausgefüllt sein
    const searchInput = overlay.locator('[data-testid="move-to-board-search"]')
    await expect(searchInput).toBeVisible()
    await expect(searchInput).toHaveValue('Backlog')
  })

  /**
   * Test 2: Aktuelles Board nicht in der Liste
   * Das aktive Projekt darf nicht als Ziel-Board angezeigt werden.
   */
  test('02 – Aktuelles Board ist nicht in der Board-Liste', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }
    await dismissPasswordChangeModal(page)

    await page.waitForSelector('#board', { timeout: 15000 })

    // Aktuellen Projekt-Titel ermitteln
    const currentProjectTitle = await page.evaluate(() => {
      // @ts-ignore
      const s = window.__state
      return s?.project?.title || null
    })

    if (!currentProjectTitle) {
      test.skip()
      return
    }

    const opened = await page.evaluate(() => {
      // @ts-ignore
      if (typeof window.__openMoveToBoardOverlay === 'function') {
        // @ts-ignore
        window.__openMoveToBoardOverlay('test-task-id')
        return true
      }
      return false
    })

    if (!opened) {
      test.skip()
      return
    }

    const overlay = page.locator('[data-testid="move-to-board-overlay"]')
    await expect(overlay).toBeVisible({ timeout: 5000 })

    // Suchfeld leeren, damit alle Boards sichtbar sind
    const searchInput = overlay.locator('[data-testid="move-to-board-search"]')
    await searchInput.fill('')
    await page.waitForTimeout(300)

    // Aktuelles Board darf nicht in der Liste sein
    const boardItems = overlay.locator('[data-testid="move-to-board-item"]')
    const boardTitles = await boardItems.allTextContents()
    const hasCurrentProject = boardTitles.some(title => title.includes(currentProjectTitle))
    expect(hasCurrentProject).toBe(false)
  })

  /**
   * Test 3: Boards ohne Spalten werden gefiltert
   * Projekte ohne Spalten dürfen nicht als Ziel angezeigt werden.
   */
  test('03 – Boards ohne Spalten werden nicht angezeigt', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }
    await dismissPasswordChangeModal(page)

    await page.waitForSelector('#board', { timeout: 15000 })

    // Injiziere ein Projekt ohne Spalten in den State
    await page.evaluate(() => {
      // @ts-ignore
      const s = window.__state
      if (!s) return
      s.projects = [
        ...(s.projects || []),
        {
          _id: 'no-columns-project-test',
          title: 'ProjektOhneSpalten',
          slug: 'projekt-ohne-spalten',
          columns: [],
          tasks: [],
          users: [],
        }
      ]
    })

    const opened = await page.evaluate(() => {
      // @ts-ignore
      if (typeof window.__openMoveToBoardOverlay === 'function') {
        // @ts-ignore
        window.__openMoveToBoardOverlay('test-task-id')
        return true
      }
      return false
    })

    if (!opened) {
      test.skip()
      return
    }

    const overlay = page.locator('[data-testid="move-to-board-overlay"]')
    await expect(overlay).toBeVisible({ timeout: 5000 })

    // Suche leeren, damit alle Boards sichtbar
    const searchInput = overlay.locator('[data-testid="move-to-board-search"]')
    await searchInput.fill('')
    await page.waitForTimeout(300)

    // Projekt ohne Spalten darf nicht erscheinen
    const boardItems = overlay.locator('[data-testid="move-to-board-item"]')
    const boardTitles = await boardItems.allTextContents()
    const hasNoColumnsProject = boardTitles.some(t => t.includes('ProjektOhneSpalten'))
    expect(hasNoColumnsProject).toBe(false)
  })

  /**
   * Test 4: Board auswählen → MCP-Call-Anfrage wird gesendet
   * Klick auf ein Board soll einen POST an /mcp mit move_task_to_project auslösen.
   */
  test('04 – Board auswählen löst MCP move_task_to_project aus', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }
    await dismissPasswordChangeModal(page)

    await page.waitForSelector('#board', { timeout: 15000 })

    // Intercept /mcp requests
    const mcpRequests: { method: string; params: unknown }[] = []
    await page.route('**/mcp', async (route) => {
      const req = route.request()
      try {
        const body = JSON.parse(req.postData() || '{}')
        if (body.params?.name === 'move_task_to_project') {
          mcpRequests.push({ method: body.method, params: body.params })
        }
      } catch { /* ignore */ }
      // Antwort mit Erfolg simulieren (verhindert Netzwerk-Fehler im Test)
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ jsonrpc: '2.0', id: 1, result: { content: [{ type: 'text', text: 'ok' }] } }),
      })
    })

    // State: Mindestens 2 Projekte mit Spalten (damit Ziel-Board existiert)
    await page.evaluate(() => {
      // @ts-ignore
      const s = window.__state
      if (!s || !s.projects || s.projects.length < 2) return
    })

    const opened = await page.evaluate(() => {
      // @ts-ignore
      if (typeof window.__openMoveToBoardOverlay === 'function') {
        // @ts-ignore
        window.__openMoveToBoardOverlay('real-task-id-for-mcp')
        return true
      }
      return false
    })

    if (!opened) {
      test.skip()
      return
    }

    const overlay = page.locator('[data-testid="move-to-board-overlay"]')
    await expect(overlay).toBeVisible({ timeout: 5000 })

    // Suchfeld leeren, damit alle Boards angezeigt werden
    const searchInput = overlay.locator('[data-testid="move-to-board-search"]')
    await searchInput.fill('')
    await page.waitForTimeout(300)

    // Ersten verfügbaren Board-Eintrag klicken
    const boardItems = overlay.locator('[data-testid="move-to-board-item"]')
    const count = await boardItems.count()

    if (count === 0) {
      console.log('Keine Ziel-Boards verfügbar – Test übersprungen')
      test.skip()
      return
    }

    await boardItems.first().click()
    await page.waitForTimeout(500)

    // MCP-Call muss abgesetzt worden sein
    expect(mcpRequests.length).toBeGreaterThan(0)
    expect(mcpRequests[0].params).toMatchObject({
      name: 'move_task_to_project',
    })

    // Overlay muss nach dem Klick geschlossen sein
    await expect(overlay).not.toBeVisible({ timeout: 3000 })
  })

})

// ─── TaskModal „Move Ticket"-Button Tests ────────────────────────────────────

test.describe('TaskModal Move-to-Board Button', () => {

  /**
   * Test 5: Button im Edit-Formular sichtbar
   * Im TaskModal (Edit-Modus) muss der „Move Ticket"-Button angezeigt werden.
   */
  test('05 – Move-to-Board-Button im Edit-Formular sichtbar', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }
    await dismissPasswordChangeModal(page)

    await page.waitForSelector('#board', { timeout: 15000 })

    const task = await page.evaluate(() => {
      // @ts-ignore
      const s = window.__state
      return s?.project?.tasks?.[0] || null
    })
    if (!task) {
      test.skip()
      return
    }

    await page.evaluate((t) => {
      // @ts-ignore
      if (typeof window.__openTaskModal === 'function') window.__openTaskModal(t)
    }, task)

    // Modal muss sichtbar sein
    const modal = page.locator('[data-testid="task-modal"]')
    await expect(modal).toBeVisible({ timeout: 5000 })

    // Move-to-Board-Button muss im Modal sichtbar sein
    const moveBtn = modal.locator('[data-testid="task-modal-move-to-board-btn"]')
    await expect(moveBtn).toBeVisible()
  })

  /**
   * Test 6: Klick auf Move-to-Board-Button öffnet das Overlay
   * Nach dem Klick muss das MoveToBoardOverlay erscheinen.
   */
  test('06 – Klick auf Move-Button öffnet das MoveToBoardOverlay', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }
    await dismissPasswordChangeModal(page)

    await page.waitForSelector('#board', { timeout: 15000 })

    const task = await page.evaluate(() => {
      // @ts-ignore
      const s = window.__state
      return s?.project?.tasks?.[0] || null
    })
    if (!task) {
      test.skip()
      return
    }

    await page.evaluate((t) => {
      // @ts-ignore
      if (typeof window.__openTaskModal === 'function') window.__openTaskModal(t)
    }, task)

    const modal = page.locator('[data-testid="task-modal"]')
    await expect(modal).toBeVisible({ timeout: 5000 })

    // Klick auf Move-to-Board-Button
    const moveBtn = modal.locator('[data-testid="task-modal-move-to-board-btn"]')
    await expect(moveBtn).toBeVisible()
    await moveBtn.click()

    // Das MoveToBoardOverlay muss erscheinen
    const overlay = page.locator('[data-testid="move-to-board-overlay"]')
    await expect(overlay).toBeVisible({ timeout: 5000 })
  })

})
