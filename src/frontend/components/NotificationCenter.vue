<script setup lang="ts">
/**
 * NotificationCenter.vue — Persistiertes Benachrichtigungs-Center.
 *
 * Zeigt alle gespeicherten Ticket-Events als Liste an (neueste zuerst).
 * Klick auf eine Benachrichtigung scrollt zum Task im Board und hebt ihn hervor.
 */
import { reactive, computed, onMounted, onUnmounted } from 'vue'
import { notificationHistoryService } from '../services/notification-history-service'
import type { NotificationEntry } from '../services/notification-history-service'
import { state } from '../state'

// Reaktiver State des Services direkt nutzen
const histState = notificationHistoryService.state

// Ungelesene Einträge als Badge
const unreadCount = computed(() => histState.unreadCount)

/** Notification-Center öffnen/schließen. */
function toggle(): void {
  notificationHistoryService.toggle()
}

/** Beim Klick außerhalb schließen. */
function handleOutsideClick(e: MouseEvent): void {
  const el = document.getElementById('notification-center')
  if (el && !el.contains(e.target as Node)) {
    notificationHistoryService.close()
  }
}

onMounted(() => {
  notificationHistoryService.load()
  document.addEventListener('mousedown', handleOutsideClick)
})

onUnmounted(() => {
  document.removeEventListener('mousedown', handleOutsideClick)
})

/**
 * Klick auf eine Benachrichtigung:
 * - Wenn das Ticket im aktuellen Board ist → scrollen + hervorheben
 * - Sonst: Toast mit Hinweis
 */
function handleNotificationClick(entry: NotificationEntry): void {
  if (!state.project) return

  if (entry.project_id !== state.project._id) {
    // Ticket gehört zu einem anderen Board — ausgegraut anzeigen
    return
  }

  // Panel schließen
  notificationHistoryService.close()

  // Task im Board hervorheben und hinein-scrollen
  highlightTask(entry.task_id)
}

/** Task-Karte im Board finden, scrollen und kurz hervorheben. */
function highlightTask(taskId: string): void {
  // Alle Task-Karten durchsuchen (data-task-id Attribut)
  const cards = document.querySelectorAll<HTMLElement>(`[data-task-id="${taskId}"]`)
  if (cards.length === 0) return

  const card = cards[0]
  card.scrollIntoView({ behavior: 'smooth', block: 'center' })

  // Highlight-Klasse hinzufügen und nach 2 Sekunden entfernen
  card.classList.add('notification-highlight')
  setTimeout(() => {
    card.classList.remove('notification-highlight')
  }, 2000)
}

/** Prüft ob das Ticket zum aktuellen Board gehört. */
function isCurrentBoard(entry: NotificationEntry): boolean {
  return !!state.project && entry.project_id === state.project._id
}

/** Einzelne Notification entfernen. */
async function removeEntry(entry: NotificationEntry, e: MouseEvent): Promise<void> {
  e.stopPropagation()
  await notificationHistoryService.remove(entry.id)
}

/** Alle Notifications löschen. */
async function clearAll(): Promise<void> {
  await notificationHistoryService.clearAll()
}

/** Icon für Event-Typ. */
function eventIcon(type: string): string {
  switch (type) {
    case 'task_created': return '✦'
    case 'task_moved': return '⇢'
    case 'task_updated': return '✎'
    case 'task_commented': return '💬'
    case 'task_deleted': return '✗'
    default: return '•'
  }
}
</script>

<template>
  <div id="notification-center" class="relative flex-shrink-0">
    <!-- Glocken-Button mit Badge -->
    <button
      class="bg-transparent border rounded-md cursor-pointer font-sans text-xs px-2.5 py-1 transition-all flex-shrink-0 relative"
      :class="histState.isOpen
        ? 'border-accent text-accent bg-accent/10'
        : 'border-border text-text-dim hover:border-accent hover:text-accent'"
      title="Benachrichtigungen"
      @click="toggle"
    >
      🔔
      <span
        v-if="unreadCount > 0"
        class="absolute -top-1.5 -right-1.5 bg-red-500 text-white text-[10px] font-bold rounded-full min-w-[16px] h-4 flex items-center justify-center px-0.5 leading-none"
      >{{ unreadCount > 9 ? '9+' : unreadCount }}</span>
    </button>

    <!-- Dropdown-Panel -->
    <Transition name="notification-panel">
      <div
        v-if="histState.isOpen"
        class="absolute top-full right-0 mt-2 z-[3000] bg-surface border border-border rounded-md shadow-[0_8px_24px_rgba(0,0,0,0.4)] w-[340px] max-w-[calc(100vw-24px)] flex flex-col"
        style="max-height: 480px"
      >
        <!-- Header -->
        <div class="flex items-center justify-between px-3 py-2 border-b border-border flex-shrink-0">
          <span class="font-mono text-xs text-text font-semibold tracking-wide">Benachrichtigungen</span>
          <button
            v-if="histState.entries.length > 0"
            class="text-text-dim text-[11px] hover:text-danger transition-colors bg-transparent border-0 cursor-pointer"
            @click="clearAll"
            title="Alle löschen"
          >Alle löschen</button>
        </div>

        <!-- Lade-Indikator -->
        <div v-if="histState.isLoading" class="px-4 py-6 text-center text-text-dim text-xs">
          Lade…
        </div>

        <!-- Leere Liste -->
        <div v-else-if="histState.entries.length === 0" class="px-4 py-8 text-center text-text-dim text-xs">
          Keine Benachrichtigungen
        </div>

        <!-- Notification-Liste -->
        <ul v-else class="overflow-y-auto flex-1 py-1 m-0 p-0 list-none">
          <li
            v-for="entry in histState.entries"
            :key="entry.id"
            class="group flex items-start gap-2.5 px-3 py-2.5 border-b border-border/50 last:border-0 cursor-pointer transition-colors"
            :class="[
              isCurrentBoard(entry)
                ? 'hover:bg-accent/10'
                : 'opacity-50 cursor-default',
              !entry.read ? 'bg-accent/5' : ''
            ]"
            @click="handleNotificationClick(entry)"
          >
            <!-- Event-Icon -->
            <span class="text-accent text-base mt-0.5 flex-shrink-0 font-mono select-none">
              {{ eventIcon(entry.event_type) }}
            </span>

            <!-- Inhalt -->
            <div class="flex-1 min-w-0">
              <div class="text-text text-[12px] font-medium truncate leading-tight">
                {{ entry.task_title }}
              </div>
              <div class="text-text-dim text-[11px] mt-0.5 leading-tight">
                {{ notificationHistoryService.formatEventType(entry.event_type) }}
                <span v-if="entry.actor"> · {{ entry.actor }}</span>
              </div>
              <div class="text-text-dim/60 text-[10px] mt-0.5">
                {{ notificationHistoryService.formatRelativeTime(entry.created_at) }}
                <span v-if="!isCurrentBoard(entry)" class="ml-1 text-yellow-500/70">(anderes Board)</span>
              </div>
            </div>

            <!-- Löschen-Button -->
            <button
              class="opacity-0 group-hover:opacity-100 text-text-dim hover:text-danger transition-all bg-transparent border-0 cursor-pointer text-base leading-none flex-shrink-0 mt-0.5"
              title="Entfernen"
              @click="removeEntry(entry, $event)"
            >×</button>
          </li>
        </ul>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
/* Einblend-Animation für das Panel */
.notification-panel-enter-active,
.notification-panel-leave-active {
  transition: opacity 0.15s ease, transform 0.15s ease;
}
.notification-panel-enter-from,
.notification-panel-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}
</style>
