<script setup lang="ts">
// MoveToBoardOverlay: Modal zum Verschieben eines Tasks auf ein anderes Board.
// Lädt beim Öffnen alle Projekte, filtert das aktuelle Board und Boards ohne Spalten,
// und ruft bei Auswahl den MCP-Endpunkt `move_task_to_project` auf.

import { ref, computed, onMounted, onUnmounted } from 'vue'
import type { ProjectDoc } from '../types'
import { state } from '../state'
import api from '../api'
import { t } from '../i18n'
import { useToast } from 'vue-toastification'

const toast = useToast()

const isOpen = ref(false)
const loading = ref(false)
const moving = ref(false)
const allProjects = ref<ProjectDoc[]>([])
const searchQuery = ref('')
const taskId = ref<string>('')

/** Öffnet das Overlay für den angegebenen Task. */
async function open(tid: string) {
  taskId.value = tid
  searchQuery.value = ''
  isOpen.value = true
  await loadProjects()
}

function close() {
  isOpen.value = false
  taskId.value = ''
  searchQuery.value = ''
  allProjects.value = []
}

async function loadProjects() {
  loading.value = true
  try {
    const projects = await api.get<ProjectDoc[]>('/api/projects')
    allProjects.value = projects
  } catch (err) {
    console.error('[MoveToBoardOverlay] load failed:', err)
    toast.error(t('moveToBoard.errorLoading'))
  } finally {
    loading.value = false
  }
}

/** Filtert: aktuelles Board + Boards ohne Spalten heraus. Gepinnte Boards zuerst. */
const filteredProjects = computed(() => {
  const currentId = state.project?._id
  const q = searchQuery.value.toLowerCase()

  const filtered = allProjects.value.filter(p => {
    if (p._id === currentId) return false
    if (!p.columns || p.columns.length === 0) return false
    if (q && !p.title.toLowerCase().includes(q)) return false
    return true
  })

  // Gepinnte Boards oben
  return [...filtered].sort((a, b) => {
    if (a.pinned && !b.pinned) return -1
    if (!a.pinned && b.pinned) return 1
    return 0
  })
})

/** Zählt Tasks eines Projekts (nur sichtbare Spalten). */
function taskCount(project: ProjectDoc): number {
  if (!project.tasks) return 0
  const visibleColIds = new Set(
    (project.columns || []).filter(c => !c.hidden).map(c => c.id)
  )
  return project.tasks.filter(task => visibleColIds.has(task.column_id)).length
}

/** Klick auf Board: MCP-Tool `move_task_to_project` aufrufen. */
async function moveToBoard(targetProject: ProjectDoc) {
  if (moving.value || !taskId.value) return
  moving.value = true

  try {
    const origin = window.location.origin
    const body = {
      jsonrpc: '2.0',
      method: 'tools/call',
      params: {
        name: 'move_task_to_project',
        arguments: {
          task_id: taskId.value,
          source_project_id: state.project?._id,
          target_project_id: targetProject._id,
        },
      },
      id: Date.now(),
    }

    const response = await fetch(`${origin}/mcp`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    })

    if (!response.ok) {
      throw new Error(`${response.status} ${response.statusText}`)
    }

    const result = await response.json()

    if (result.error) {
      throw new Error(result.error.message || String(result.error))
    }

    toast.success(t('moveToBoard.success', { board: targetProject.title }))
    close()

    // Board-Refresh triggern damit verschobener Task verschwindet
    if (typeof (window as any).__kanbanRefresh === 'function') {
      ;(window as any).__kanbanRefresh()
    }
  } catch (err) {
    console.error('[MoveToBoardOverlay] move failed:', err)
    toast.error(t('moveToBoard.errorMoving'))
  } finally {
    moving.value = false
  }
}

/** Escape-Taste schließt das Overlay. */
function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape' && isOpen.value) close()
}

onMounted(() => {
  window.addEventListener('keydown', onKeydown)
  // Globale Bridge: __openMoveToBoardOverlay(taskId)
  ;(window as any).__openMoveToBoardOverlay = (tid: string) => open(tid)
})

onUnmounted(() => {
  window.removeEventListener('keydown', onKeydown)
  delete (window as any).__openMoveToBoardOverlay
})

defineExpose({ open, close })
</script>

<template>
  <Teleport to="body">
    <!-- Backdrop -->
    <Transition name="mtb-overlay">
      <div
        v-if="isOpen"
        class="fixed inset-0 bg-black/60 backdrop-blur-[2px] z-[1100]"
        @click="close"
      />
    </Transition>

    <!-- Modal -->
    <Transition name="mtb-modal">
      <div
        v-if="isOpen"
        data-testid="move-to-board-overlay"
        class="fixed left-1/2 top-[15%] -translate-x-1/2 z-[1150]
               w-[480px] max-w-[95vw]
               bg-surface border border-border rounded-lg
               shadow-[0_16px_48px_rgba(0,0,0,0.5)]
               flex flex-col"
        @click.stop
      >
        <!-- Header -->
        <div class="flex items-center justify-between px-5 py-4 border-b border-border flex-shrink-0">
          <h2 class="font-mono text-[13px] font-semibold tracking-wide uppercase text-text-dim">
            {{ t('moveToBoard.title') }}
          </h2>
          <button
            class="bg-transparent border-none text-text-dim cursor-pointer text-base px-1.5 py-0.5 hover:text-text transition-colors"
            :title="t('close')"
            @click="close"
          >&#10005;</button>
        </div>

        <!-- Suchfeld -->
        <div class="px-4 pt-3.5 pb-2 flex-shrink-0">
          <input
            v-model="searchQuery"
            type="text"
            data-testid="move-to-board-search"
            :placeholder="t('moveToBoard.searchPlaceholder')"
            autofocus
            class="w-full bg-surface-2 border border-border rounded-md text-text text-[13px] px-2.5 py-1.5 outline-none font-sans transition-colors focus:border-accent placeholder:text-text-dim"
          />
        </div>

        <!-- Board-Liste -->
        <div class="flex-1 overflow-y-auto max-h-[360px] px-2 pb-3 flex flex-col gap-1">
          <!-- Loading-Spinner -->
          <div v-if="loading" class="flex items-center justify-center py-10 gap-2 text-text-dim text-sm font-mono">
            <span class="animate-spin inline-block w-4 h-4 border-2 border-current border-t-transparent rounded-full"></span>
            {{ t('moveToBoard.loading') }}
          </div>

          <!-- Leerer State -->
          <div
            v-else-if="filteredProjects.length === 0"
            class="flex flex-col items-center justify-center py-10 gap-2 opacity-50"
          >
            <span class="text-2xl">&#128247;</span>
            <span class="text-sm text-text-dim">{{ t('moveToBoard.empty') }}</span>
          </div>

          <!-- Board-Einträge -->
          <button
            v-for="project in filteredProjects"
            :key="project._id"
            data-testid="move-to-board-item"
            :disabled="moving"
            class="w-full text-left bg-surface-2 border border-border rounded-md px-3.5 py-2.5
                   flex items-center justify-between gap-3
                   cursor-pointer transition-all
                   hover:border-accent hover:bg-accent/5
                   disabled:opacity-40 disabled:cursor-not-allowed
                   focus:outline-none focus:border-accent"
            @click="moveToBoard(project)"
          >
            <span class="flex items-center gap-1.5 flex-1 min-w-0">
              <span v-if="project.pinned" class="text-accent flex-shrink-0" title="Gepinnt">📌</span>
              <span class="text-[13px] font-semibold text-text leading-snug truncate">{{ project.title }}</span>
            </span>
            <span class="font-mono text-[11px] text-text-dim bg-surface border border-border rounded-full px-2 py-px flex-shrink-0">
              {{ taskCount(project) }} Tasks
            </span>
          </button>
        </div>

        <!-- Moving-Spinner (ganzflächig) -->
        <Transition name="mtb-spinner">
          <div
            v-if="moving"
            class="absolute inset-0 bg-surface/80 flex items-center justify-center rounded-lg z-10"
          >
            <span class="animate-spin inline-block w-6 h-6 border-2 border-accent border-t-transparent rounded-full"></span>
          </div>
        </Transition>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.mtb-overlay-enter-active,
.mtb-overlay-leave-active {
  transition: opacity 0.2s ease;
}
.mtb-overlay-enter-from,
.mtb-overlay-leave-to {
  opacity: 0;
}

.mtb-modal-enter-active,
.mtb-modal-leave-active {
  transition: opacity 0.18s ease, transform 0.18s ease;
}
.mtb-modal-enter-from,
.mtb-modal-leave-to {
  opacity: 0;
  transform: translateX(-50%) translateY(-8px) scale(0.97);
}

.mtb-spinner-enter-active,
.mtb-spinner-leave-active {
  transition: opacity 0.15s ease;
}
.mtb-spinner-enter-from,
.mtb-spinner-leave-to {
  opacity: 0;
}
</style>
