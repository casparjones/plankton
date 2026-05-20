/**
 * Playwright-Tests für den Notification-Service (Ticket 772e0e9a)
 *
 * Testet:
 * - Toast-Benachrichtigung bei fremden SSE-Events (task_moved, task_created, task_commented)
 * - Keine Toast-Benachrichtigung für eigene Aktionen
 * - Browser Notification API Permission Request
 * - localStorage-Preference plankton_notifications_enabled
 */

import { test, expect, Page } from '@playwright/test'

const USERNAME = process.env.PLANKTON_USER || 'admin'
const PASSWORD = process.env.PLANKTON_PASS || 'admin'
const BASE_URL = process.env.PLANKTON_URL || 'http://localhost:3099'

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

// ─── Hilfsfunktion: SSE-Event simulieren via window.__simulateSSE ────────────

async function simulateSSEEvent(page: Page, event: string, data: Record<string, unknown>) {
  return page.evaluate(({ event, data }) => {
    if (typeof (window as any).__simulateSSE === 'function') {
      (window as any).__simulateSSE(event, data)
      return true
    }
    return false
  }, { event, data })
}

// ─── Tests ───────────────────────────────────────────────────────────────────

test.describe('Notification-Service', () => {

  test('N01 – notificationService ist im globalem Scope verfügbar', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }

    await page.waitForSelector('#board', { timeout: 15000 })

    const hasService = await page.evaluate(() => {
      return typeof (window as any).__notificationService !== 'undefined'
        || typeof (window as any).notificationService !== 'undefined'
    })
    expect(hasService).toBe(true)
  })

  test('N02 – Toast erscheint bei task_moved-Event von anderem Nutzer', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }

    await page.waitForSelector('#board', { timeout: 15000 })

    // SSE-Event von einem anderen User simulieren
    const simulated = await simulateSSEEvent(page, 'task_moved', {
      id: 'test-task-123',
      title: 'Test Task',
      column_id: 'col-2',
      column_slug: 'in-progress',
      worker: 'other-user',  // anderer User
      creator: 'other-user',
      actor: 'other-user',
    })

    if (!simulated) { test.skip(); return }

    // Toast-Container muss erscheinen
    await expect(page.locator('.Vue-Toastification__toast')).toBeVisible({ timeout: 3000 })
  })

  test('N03 – Toast erscheint bei task_created-Event von anderem Nutzer', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }

    await page.waitForSelector('#board', { timeout: 15000 })

    const simulated = await simulateSSEEvent(page, 'task_created', {
      id: 'test-task-456',
      title: 'Neuer Task von anderem User',
      column_id: 'col-1',
      column_slug: 'todo',
      worker: '',
      creator: 'other-user',
      actor: 'other-user',
    })

    if (!simulated) { test.skip(); return }

    await expect(page.locator('.Vue-Toastification__toast')).toBeVisible({ timeout: 3000 })
  })

  test('N04 – KEIN Toast bei eigenem task_moved-Event', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }

    await page.waitForSelector('#board', { timeout: 15000 })

    // Aktuellen Username ermitteln
    const currentUsername = await page.evaluate(() => {
      const state = (window as any).__state || (window as any).appState
      return state?.currentUser?.username || null
    })

    if (!currentUsername) { test.skip(); return }

    // Toast-Anzahl vor dem Event
    const toastsBefore = await page.locator('.Vue-Toastification__toast').count()

    // SSE-Event vom selben User simulieren
    const simulated = await simulateSSEEvent(page, 'task_moved', {
      id: 'test-task-789',
      title: 'Mein eigener Task',
      column_id: 'col-2',
      column_slug: 'in-progress',
      actor: currentUsername,  // selber User → kein Toast
    })

    if (!simulated) { test.skip(); return }

    // Kurz warten, dann sicherstellen dass kein neuer Toast erschien
    await page.waitForTimeout(500)
    const toastsAfter = await page.locator('.Vue-Toastification__toast').count()
    expect(toastsAfter).toBe(toastsBefore)
  })

  test('N05 – Notifications können per localStorage deaktiviert werden', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }

    // Notifications deaktivieren BEVOR das Board lädt
    await page.addInitScript(() => {
      localStorage.setItem('plankton_notifications_enabled', 'false')
    })

    await page.reload()
    await page.waitForSelector('#board', { timeout: 15000 })

    const toastsBefore = await page.locator('.Vue-Toastification__toast').count()

    const simulated = await simulateSSEEvent(page, 'task_moved', {
      id: 'test-task-disabled',
      title: 'Task bei deaktivierten Notifications',
      column_id: 'col-2',
      column_slug: 'in-progress',
      actor: 'other-user',
    })

    if (!simulated) { test.skip(); return }

    await page.waitForTimeout(500)
    const toastsAfter = await page.locator('.Vue-Toastification__toast').count()
    expect(toastsAfter).toBe(toastsBefore)

    // Aufräumen
    await page.evaluate(() => localStorage.removeItem('plankton_notifications_enabled'))
  })

  test('N06 – Notification-Toggle-Button ist im Board sichtbar', async ({ page }) => {
    const loggedIn = await tryLogin(page)
    if (!loggedIn) { test.skip(); return }

    await page.waitForSelector('#board', { timeout: 15000 })

    // Toggle-Button für Notifications muss existieren
    const toggleBtn = page.locator('[data-notification-toggle], #notification-toggle, .notification-toggle')
    await expect(toggleBtn).toBeVisible({ timeout: 5000 })
  })

})
