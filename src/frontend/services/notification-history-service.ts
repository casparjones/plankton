/**
 * Notification-History-Service für Plankton.
 *
 * Lädt persistierte Benachrichtigungen vom Backend (/api/notifications)
 * und aktualisiert die Liste bei neuen SSE-Events.
 *
 * Jede Benachrichtigung enthält task_id, project_id und event_type —
 * damit kann das Frontend zu einem Task navigieren und ihn hervorheben.
 */

import { reactive } from 'vue'
import api from '../api'

/** Event-Typ aus dem Backend (entspricht NotificationEventType in Rust). */
export type NotificationEventType =
  | 'task_created'
  | 'task_moved'
  | 'task_updated'
  | 'task_commented'
  | 'task_deleted'

/** Eine einzelne Benachrichtigung vom Backend. */
export interface NotificationEntry {
  id: string
  event_type: NotificationEventType
  task_id: string
  task_title: string
  project_id: string
  actor: string | null
  read: boolean
  created_at: string
}

/** Reaktiver State für das Notification-Center. */
interface NotificationHistoryState {
  entries: NotificationEntry[]
  unreadCount: number
  isLoading: boolean
  isOpen: boolean
}

// Vue-reaktiver State — muss reactive() sein damit Komponenten Änderungen erkennen
const _state = reactive<NotificationHistoryState>({
  entries: [],
  unreadCount: 0,
  isLoading: false,
  isOpen: false,
})

class NotificationHistoryService {
  /** Öffentlicher State — wird direkt von Vue-Komponenten via reactive() genutzt. */
  readonly state = _state

  /** Benachrichtigungen vom Backend laden. */
  async load(): Promise<void> {
    _state.isLoading = true
    try {
      const entries = await api.get<NotificationEntry[]>('/api/notifications')
      _state.entries = entries
      _state.unreadCount = entries.filter((e) => !e.read).length
    } catch (err) {
      console.warn('[NotificationHistory] Laden fehlgeschlagen:', err)
    } finally {
      _state.isLoading = false
    }
  }

  /** Neuen Eintrag vorne einfügen (bei SSE-Events aufrufen). */
  prepend(entry: NotificationEntry): void {
    _state.entries.unshift(entry)
    if (!entry.read) {
      _state.unreadCount++
    }
  }

  /** Einzelne Notification löschen. */
  async remove(id: string): Promise<void> {
    try {
      await api.del(`/api/notifications/${id}`)
      const idx = _state.entries.findIndex((e) => e.id === id)
      if (idx >= 0) {
        if (!_state.entries[idx].read) _state.unreadCount--
        _state.entries.splice(idx, 1)
      }
    } catch (err) {
      console.warn('[NotificationHistory] Löschen fehlgeschlagen:', err)
    }
  }

  /** Alle Notifications löschen. */
  async clearAll(): Promise<void> {
    try {
      await api.del('/api/notifications')
      _state.entries = []
      _state.unreadCount = 0
    } catch (err) {
      console.warn('[NotificationHistory] Clear-All fehlgeschlagen:', err)
    }
  }

  /** Alle als gelesen markieren (lokal, kein API-Call nötig). */
  markAllRead(): void {
    for (const e of _state.entries) {
      e.read = true
    }
    _state.unreadCount = 0
  }

  /** Panel öffnen/schließen. */
  toggle(): void {
    _state.isOpen = !_state.isOpen
    if (_state.isOpen) {
      this.markAllRead()
    }
  }

  /** Panel schließen. */
  close(): void {
    _state.isOpen = false
  }

  /** Lesbare Beschreibung eines Notification-Event-Typs. */
  formatEventType(type: NotificationEventType): string {
    switch (type) {
      case 'task_created':
        return 'erstellt'
      case 'task_moved':
        return 'verschoben'
      case 'task_updated':
        return 'aktualisiert'
      case 'task_commented':
        return 'kommentiert'
      case 'task_deleted':
        return 'gelöscht'
      default:
        return type
    }
  }

  /** Relativen Zeitstring erzeugen (z.B. "vor 5 Minuten"). */
  formatRelativeTime(isoString: string): string {
    const diff = Date.now() - new Date(isoString).getTime()
    const minutes = Math.floor(diff / 60_000)
    if (minutes < 1) return 'gerade eben'
    if (minutes === 1) return 'vor 1 Minute'
    if (minutes < 60) return `vor ${minutes} Minuten`
    const hours = Math.floor(minutes / 60)
    if (hours === 1) return 'vor 1 Stunde'
    if (hours < 24) return `vor ${hours} Stunden`
    return 'vor mehr als einem Tag'
  }
}

/** Singleton-Instanz, global verfügbar. */
export const notificationHistoryService = new NotificationHistoryService()

// Global für Debugging
;(window as any).__notificationHistoryService = notificationHistoryService
