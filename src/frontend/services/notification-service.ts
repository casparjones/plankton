/**
 * Notification-Service für Plankton.
 *
 * Verarbeitet SSE-Events und zeigt Toast-Benachrichtigungen
 * sowie optionale Browser-Notifications an, wenn ein anderer
 * Nutzer einen Task ändert, erstellt oder kommentiert.
 *
 * localStorage-Key: plankton_notifications_enabled (default: true)
 */

import { toast } from '../toast'
import { state } from '../state'
import { t } from '../i18n'
import { POSITION } from 'vue-toastification'

const STORAGE_KEY = 'plankton_notifications_enabled'

export interface SSENotificationPayload {
  /** Granulares Event aus dem SSE-Stream */
  event: string
  /** Task-Daten oder Meta-Daten */
  data: {
    id?: string
    title?: string
    column_id?: string
    column_slug?: string
    creator?: string
    worker?: string
    /** Wer die Aktion ausgelöst hat (optional, vom Backend befüllt) */
    actor?: string
    task_id?: string
    [key: string]: unknown
  }
}

class NotificationService {
  private _browserPermission: NotificationPermission = 'default'

  constructor() {
    // Browser-Permission-Status laden
    if ('Notification' in window) {
      this._browserPermission = Notification.permission
    }
  }

  // ── Einstellungen ──────────────────────────────────────────────────────────

  /** Sind Benachrichtigungen aktiviert? */
  isEnabled(): boolean {
    const stored = localStorage.getItem(STORAGE_KEY)
    // Default: aktiviert (null = nicht gesetzt)
    return stored !== 'false'
  }

  /** Benachrichtigungen aktivieren / deaktivieren. */
  setEnabled(enabled: boolean): void {
    localStorage.setItem(STORAGE_KEY, enabled ? 'true' : 'false')
  }

  /** Umschalten. */
  toggle(): boolean {
    const next = !this.isEnabled()
    this.setEnabled(next)
    return next
  }

  // ── Browser Notification API ───────────────────────────────────────────────

  /** Permission einmalig anfragen. Zeigt Toast-Fallback wenn verweigert. */
  async requestBrowserPermission(): Promise<NotificationPermission> {
    if (!('Notification' in window)) return 'denied'
    if (Notification.permission === 'granted') return 'granted'
    if (Notification.permission === 'denied') return 'denied'

    try {
      const result = await Notification.requestPermission()
      this._browserPermission = result
      return result
    } catch {
      return 'denied'
    }
  }

  /** Browser-Notification senden (Fallback auf Toast wenn Permission fehlt). */
  private _sendBrowserNotification(title: string, body: string): void {
    if (!('Notification' in window)) return
    if (Notification.permission !== 'granted') return
    try {
      new Notification(title, {
        body,
        icon: '/favicon.png',
        tag: 'plankton-task-update',
      })
    } catch {
      // Ignorieren falls Notification-Konstruktor fehlschlägt (z.B. in manchen Browsern)
    }
  }

  // ── Haupt-Einstiegspunkt ───────────────────────────────────────────────────

  /**
   * Prüft ein SSE-Event und zeigt ggf. eine Benachrichtigung an.
   * Wird von sse-service.ts nach jeder Event-Verarbeitung aufgerufen.
   */
  notify(payload: SSENotificationPayload): void {
    if (!this.isEnabled()) return

    // Eigene Aktionen nicht benachrichtigen
    if (this._isOwnAction(payload)) return

    switch (payload.event) {
      case 'task_moved':
        this._notifyTaskMoved(payload)
        break
      case 'task_created':
        this._notifyTaskCreated(payload)
        break
      case 'task_updated':
        this._notifyTaskUpdated(payload)
        break
      case 'task_commented':
        this._notifyTaskCommented(payload)
        break
      // Andere Events (task_deleted, project_update) ignorieren
    }
  }

  // ── Hilfsmethoden ─────────────────────────────────────────────────────────

  /** Prüft ob die Aktion vom aktuell eingeloggten User stammt. */
  private _isOwnAction(payload: SSENotificationPayload): boolean {
    const actor = payload.data?.actor
    if (!actor) return false
    const me = state.currentUser?.username
    if (!me) return false
    return actor === me
  }

  private _notifyTaskMoved(payload: SSENotificationPayload): void {
    const title = payload.data.title || t('notifications.unknownTask')
    const column = payload.data.column_slug || payload.data.column_id || '?'
    const actor = payload.data.actor || t('notifications.someone')
    const msg = t('notifications.taskMoved', { title, column, actor })

    toast.info(msg, {
      position: POSITION.BOTTOM_RIGHT,
      timeout: 4000,
      closeOnClick: true,
    })
    this._sendBrowserNotification('Plankton', msg)
  }

  private _notifyTaskCreated(payload: SSENotificationPayload): void {
    const title = payload.data.title || t('notifications.unknownTask')
    const actor = payload.data.actor || payload.data.creator || t('notifications.someone')
    const msg = t('notifications.taskCreated', { title, actor })

    toast.success(msg, {
      position: POSITION.BOTTOM_RIGHT,
      timeout: 4000,
      closeOnClick: true,
    })
    this._sendBrowserNotification('Plankton', msg)
  }

  private _notifyTaskUpdated(payload: SSENotificationPayload): void {
    const title = payload.data.title || t('notifications.unknownTask')
    const actor = payload.data.actor || t('notifications.someone')
    const msg = t('notifications.taskUpdated', { title, actor })

    toast.info(msg, {
      position: POSITION.BOTTOM_RIGHT,
      timeout: 3000,
      closeOnClick: true,
    })
    this._sendBrowserNotification('Plankton', msg)
  }

  private _notifyTaskCommented(payload: SSENotificationPayload): void {
    const title = payload.data.title || t('notifications.unknownTask')
    const actor = payload.data.actor || t('notifications.someone')
    const msg = t('notifications.taskCommented', { title, actor })

    toast.info(msg, {
      position: POSITION.BOTTOM_RIGHT,
      timeout: 4000,
      closeOnClick: true,
    })
    this._sendBrowserNotification('Plankton', msg)
  }
}

/** Singleton-Instanz, global verfügbar. */
export const notificationService = new NotificationService()

// Global für Tests + Debugging
;(window as any).__notificationService = notificationService
