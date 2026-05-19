<script setup lang="ts">
// Archiv-Panel: Zeigt archivierte Tasks (≥14 Tage in Done) und ermöglicht
// die Wiederherstellung in Done oder Todo.

import { ref, computed, watch } from 'vue'
import type { Task, ProjectDoc } from '../types'
import { state } from '../state'
import api from '../api'
import { t } from '../i18n'
import { useToast } from 'vue-toastification'

const toast = useToast()

const isOpen = ref(false)
const loading = ref(false)
const archivedTasks = ref<Task[]>([])
const restoringId = ref<string | null>(null)
const searchQuery = ref('')

/** Archiv-Panel öffnen und Tasks laden. */
async function open() {
  isOpen.value = true
  await loadArchive()
}

function close() {
  isOpen.value = false
  searchQuery.value = ''
}

async function loadArchive() {
  if (!state.project) return
  loading.value = true
  try {
    const project = await api.get<ProjectDoc>(`/api/projects/${state.project._id}?include_archived=true`)
    // Nur Tasks aus versteckten Spalten (= _archive) anzeigen.
    const hiddenColIds = project.columns
      .filter(c => c.hidden)
      .map(c => c.id)
    archivedTasks.value = project.tasks
      .filter(t => hiddenColIds.includes(t.column_id))
      .sort((a, b) => b.updated_at.localeCompare(a.updated_at))
  } catch (err) {
    console.error('[ArchivePanel] load failed:', err)
    toast.error(t('archive.error'))
  } finally {
    loading.value = false
  }
}

const filteredTasks = computed(() => {
  const q = searchQuery.value.toLowerCase()
  if (!q) return archivedTasks.value
  return archivedTasks.value.filter(t =>
    t.title.toLowerCase().includes(q) ||
    (t.description || '').toLowerCase().includes(q)
  )
})

/** Task wiederherstellen: in die Done- oder Todo-Spalte verschieben. */
async function restore(task: Task, target: 'done' | 'todo') {
  if (!state.project || restoringId.value) return
  restoringId.value = task.id

  const targetTitle = target === 'done' ? 'Done' : 'Todo'
  const targetCol = state.project.columns.find(c => c.title === targetTitle)
  if (!targetCol) {
    toast.error(`Column "${targetTitle}" not found`)
    restoringId.value = null
    return
  }

  try {
    // Determine order: append at end of target column.
    const tasksInCol = state.project.tasks.filter(t => t.column_id === targetCol.id)
    const order = tasksInCol.length

    await api.post(`/api/projects/${state.project._id}/tasks/${task.id}/move`, {
      column_id: targetCol.id,
      order,
    })

    // Reload state.project to reflect the restored task.
    state.project = await api.get<ProjectDoc>(`/api/projects/${state.project._id}`)

    toast.success(t('archive.restored', { title: task.title }))

    // Remove from local archive list.
    archivedTasks.value = archivedTasks.value.filter(t => t.id !== task.id)

    // Trigger board refresh.
    if (typeof (window as any).__kanbanRefresh === 'function') {
      (window as any).__kanbanRefresh()
    }
  } catch (err) {
    console.error('[ArchivePanel] restore failed:', err)
    toast.error(String(err))
  } finally {
    restoringId.value = null
  }
}

function formatDate(iso: string): string {
  if (!iso) return ''
  try {
    return new Date(iso).toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' })
  } catch {
    return iso
  }
}

// Reload when project changes.
watch(() => state.project?._id, () => {
  if (isOpen.value) loadArchive()
})

// Expose open function so AppLayout can trigger it.
defineExpose({ open, close })
</script>

<template>
  <!-- Overlay -->
  <Teleport to="body">
    <Transition name="archive-overlay">
      <div
        v-if="isOpen"
        class="fixed inset-0 bg-black/60 backdrop-blur-[2px] z-[900]"
        @click="close"
      />
    </Transition>

    <!-- Panel (slides in from right) -->
    <Transition name="archive-panel">
      <div
        v-if="isOpen"
        class="fixed right-0 top-0 h-full w-[420px] max-w-[95vw] z-[950]
               bg-surface border-l border-border
               shadow-[0_0_40px_rgba(0,0,0,0.5)]
               flex flex-col"
      >
        <!-- Header -->
        <div class="flex items-center justify-between px-5 py-4 border-b border-border flex-shrink-0">
          <h2 class="font-mono text-[13px] font-semibold tracking-wide uppercase text-text-dim flex items-center gap-2">
            <span class="text-text-dim">&#128451;</span>
            {{ t('archive.title') }}
            <span v-if="archivedTasks.length" class="bg-surface-2 border border-border rounded-full text-[10px] font-mono px-[7px] py-px text-text-dim">
              {{ archivedTasks.length }}
            </span>
          </h2>
          <button
            class="bg-transparent border-none text-text-dim cursor-pointer text-base px-1.5 py-0.5 hover:text-text transition-colors"
            :title="t('close')"
            @click="close"
          >&#10005;</button>
        </div>

        <!-- Search -->
        <div class="px-4 py-2.5 border-b border-border flex-shrink-0">
          <input
            v-model="searchQuery"
            type="text"
            :placeholder="t('board.search') + '…'"
            class="w-full bg-surface-2 border border-border rounded-md text-text text-[13px] px-2.5 py-1.5 outline-none font-sans transition-colors focus:border-accent placeholder:text-text-dim"
          />
        </div>

        <!-- Content -->
        <div class="flex-1 overflow-y-auto p-4 flex flex-col gap-2">
          <!-- Loading -->
          <div v-if="loading" class="flex items-center justify-center py-10 text-text-dim text-sm font-mono">
            {{ t('archive.loading') }}
          </div>

          <!-- Empty state -->
          <div v-else-if="filteredTasks.length === 0" class="flex flex-col items-center justify-center py-10 gap-2 opacity-50">
            <span class="text-3xl">&#128451;</span>
            <span class="text-sm text-text-dim">{{ t('archive.empty') }}</span>
          </div>

          <!-- Task list -->
          <div
            v-for="task in filteredTasks"
            :key="task.id"
            class="bg-surface-2 border border-border rounded-md p-3 flex flex-col gap-2 transition-opacity duration-150"
            :class="{ 'opacity-50': restoringId === task.id }"
          >
            <!-- Task title + type badge -->
            <div class="flex items-start gap-2">
              <span
                v-if="task.task_type === 'epic'"
                class="inline-flex items-center justify-center font-mono text-[9px] font-bold w-[18px] h-[18px] rounded-sm flex-shrink-0 bg-badge-epic-bg text-badge-epic-text border border-badge-epic-border"
                title="Epic"
              >E</span>
              <span
                v-else-if="task.task_type === 'job'"
                class="inline-flex items-center justify-center font-mono text-[9px] font-bold w-[18px] h-[18px] rounded-sm flex-shrink-0 bg-badge-job-bg text-badge-job-text border border-badge-job-border"
                title="Job"
              >J</span>
              <span class="text-[13px] font-semibold text-text leading-snug flex-1">{{ task.title }}</span>
              <span v-if="task.points" class="bg-accent-dim border border-accent rounded-full text-accent font-mono text-[10px] font-semibold px-[7px] py-px flex-shrink-0">
                {{ task.points }}
              </span>
            </div>

            <!-- Description preview -->
            <div v-if="task.description" class="text-xs text-text-dim leading-snug line-clamp-2">
              {{ task.description.substring(0, 120) }}{{ task.description.length > 120 ? '…' : '' }}
            </div>

            <!-- Labels -->
            <div v-if="(task.labels || []).length" class="flex gap-1 flex-wrap">
              <span
                v-for="label in task.labels"
                :key="label"
                class="font-mono text-[10px] px-1.5 py-px rounded-sm border border-border text-text-dim bg-surface"
              >{{ label }}</span>
            </div>

            <!-- Meta + actions -->
            <div class="flex items-center justify-between gap-2 mt-1">
              <span class="text-[11px] text-text-dim font-mono">
                {{ t('archive.archivedAt') }}: {{ formatDate(task.updated_at) }}
              </span>
              <div class="flex gap-1.5 flex-shrink-0">
                <button
                  :disabled="restoringId !== null"
                  class="bg-transparent border border-border rounded-md text-text-dim text-[11px] px-2 py-0.5 cursor-pointer font-mono transition-all hover:border-accent hover:text-accent disabled:opacity-40 disabled:cursor-not-allowed"
                  :title="t('archive.restoreToTodo')"
                  @click="restore(task, 'todo')"
                >
                  Todo
                </button>
                <button
                  :disabled="restoringId !== null"
                  class="bg-accent-dim border border-accent rounded-md text-text text-[11px] px-2 py-0.5 cursor-pointer font-mono transition-all hover:bg-accent disabled:opacity-40 disabled:cursor-not-allowed"
                  :title="t('archive.restoreToDone')"
                  @click="restore(task, 'done')"
                >
                  Done
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.archive-overlay-enter-active,
.archive-overlay-leave-active {
  transition: opacity 0.2s ease;
}
.archive-overlay-enter-from,
.archive-overlay-leave-to {
  opacity: 0;
}

.archive-panel-enter-active,
.archive-panel-leave-active {
  transition: transform 0.25s cubic-bezier(0.32, 0.72, 0, 1);
}
.archive-panel-enter-from,
.archive-panel-leave-to {
  transform: translateX(100%);
}
</style>
