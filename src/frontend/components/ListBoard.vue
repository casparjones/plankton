<script setup lang="ts">
// List-Board Komponente: rendert ein Projekt mit type="list".
// Zeigt eine einzelne Spalte (die erste sichtbare, nicht-versteckte Spalte)
// als kompakte vertikale Liste — kein Drag & Drop zwischen Spalten,
// keine Spalten-Buttons, kein Spalten-Header.

import { ref, computed, watch, nextTick } from 'vue'
import { VueDraggable } from 'vue-draggable-plus'
import { marked } from 'marked'
import type { Task, Column } from '../types'

/** Markdown → Plaintext für Task-Preview (Tags strippen) */
function stripMarkdown(text: string): string {
  const html = marked.parse(text, { async: false }) as string
  const tmp = document.createElement('div')
  tmp.innerHTML = html
  return (tmp.textContent || tmp.innerText || '').replace(/\s+/g, ' ').trim()
}

import { t } from '../i18n'
import { state } from '../state'
import api, { ApiError } from '../api'
import { useToast } from 'vue-toastification'
import { updateBulkBar } from './bulk-actions'
import { escapeHtml, labelColor } from '../utils'

const toast = useToast()

// Feste Farbzuweisung pro Worker (gleiche Logik wie KanbanBoard)
const WORKER_COLORS = [
  '#64B5F6', '#FFB74D', '#81C784', '#E57373',
  '#BA68C8', '#4DD0E1', '#FF8A65', '#AED581',
  '#F06292', '#7986CB',
]
const workerColorCache: Record<string, string> = {}

function workerBorderColor(worker: string): string {
  if (!worker) return 'var(--border)'
  const key = worker.trim().toLowerCase()
  if (!workerColorCache[key]) {
    let hash = 0
    for (let i = 0; i < key.length; i++) hash = ((hash << 5) - hash + key.charCodeAt(i)) | 0
    workerColorCache[key] = WORKER_COLORS[Math.abs(hash) % WORKER_COLORS.length]
  }
  return workerColorCache[key]
}

/** Prüft ob ein Task blockiert ist. */
function isBlocked(task: Task): boolean {
  if (!task.blocked_by?.length || !state.project) return false
  const doneCol = state.project.columns.find((c: Column) => c.title === 'Done')
  if (!doneCol) return task.blocked_by.length > 0
  return task.blocked_by.some(bid => {
    const blocker = state.project!.tasks.find((t: Task) => t.id === bid)
    return blocker && blocker.column_id !== doneCol.id
  })
}

/** Berechnet Subtask-Fortschritt für Epics. */
function subtaskProgress(task: Task): { done: number; total: number; pct: number } {
  if (!task.subtask_ids?.length || !state.project) return { done: 0, total: 0, pct: 0 }
  const doneCol = state.project.columns.find((c: Column) => c.title === 'Done')
  const total = task.subtask_ids.length
  const done = doneCol
    ? task.subtask_ids.filter(sid => {
        const sub = state.project!.tasks.find((t: Task) => t.id === sid)
        return sub && sub.column_id === doneCol.id
      }).length
    : 0
  return { done, total, pct: total > 0 ? Math.round((done / total) * 100) : 0 }
}

/**
 * Die erste sichtbare (nicht-versteckte) Spalte im List-Projekt.
 * List-Projekte haben konzeptuell nur eine Spalte die angezeigt wird.
 */
const listColumn = computed<Column | null>(() => {
  if (!state.project?.columns) return null
  return state.project.columns
    .filter((c: Column) => !c.hidden)
    .sort((a: Column, b: Column) => a.order - b.order)[0] ?? null
})

/** Tasks in der List-Spalte, sortiert nach order. */
const listTasks = ref<Task[]>([])

function refreshTasks(): void {
  if (!listColumn.value) {
    listTasks.value = []
    return
  }
  listTasks.value = (state.project?.tasks || [])
    .filter((t: Task) => t.column_id === listColumn.value!.id)
    .sort((a: Task, b: Task) => a.order - b.order)
}

watch(() => state.project, () => {
  nextTick(() => {
    refreshTasks()
    updateBulkBar()
  })
}, { deep: true, immediate: true })

/** Batch-Move persistieren (gleiche Logik wie KanbanBoard). */
function persistMoves(moves: { task_id: string; column_id: string; order: number }[]): void {
  if (!moves.length) return
  const snapshot: { task_id: string; column_id: string; order: number }[] = []
  for (const m of moves) {
    const task = state.project?.tasks.find((t: Task) => t.id === m.task_id)
    if (task) {
      snapshot.push({ task_id: task.id, column_id: task.column_id, order: task.order })
      task.column_id = m.column_id
      task.order = m.order
    }
  }
  api.post(`/api/projects/${state.project!._id}/tasks/batch-move`, { moves })
    .then(async () => {
      const p = await api.get<typeof state.project>(`/api/projects/${state.project!._id}`)
      state.project = p
    })
    .catch(err => {
      console.error('[ListBoard] batch-move failed:', err)
      if (err instanceof ApiError && err.code) {
        toast.error(t(`drag.${err.code}`, { details: err.message }), { timeout: 5000 })
      } else {
        toast.error(t('drag.moveFailed'), { timeout: 5000 })
      }
      for (const s of snapshot) {
        const task = state.project?.tasks.find((t: Task) => t.id === s.task_id)
        if (task) {
          task.column_id = s.column_id
          task.order = s.order
        }
      }
      refreshTasks()
    })
}

/** Reihenfolge innerhalb der Liste nach Drag persistieren. */
function onSortUpdate(evt: any): void {
  const from = Math.min(evt.oldIndex, evt.newIndex)
  const to = Math.max(evt.oldIndex, evt.newIndex)
  const moves: { task_id: string; column_id: string; order: number }[] = []
  const colId = listColumn.value!.id
  for (let i = from; i <= to; i++) {
    const task = listTasks.value[i]
    if (task && task.order !== i) {
      task.order = i
      moves.push({ task_id: task.id, column_id: colId, order: i })
    }
  }
  persistMoves(moves)
}

/** Checkbox-Handler für Bulk-Selection. */
function toggleTaskSelection(taskId: string, checked: boolean): void {
  if (checked) {
    state.selectedTasks.add(taskId)
  } else {
    state.selectedTasks.delete(taskId)
  }
  updateBulkBar()
}

/** Task-Klick → Detail-Ansicht öffnen. */
function handleTaskClick(task: Task, event: Event): void {
  if (state.isDragging) return
  const target = event.target as HTMLElement
  if (target.closest('.task-checkbox')) return
  if (target.closest('.task-archive-btn')) return
  // @ts-ignore
  if (typeof window.__openTaskDetail === 'function') window.__openTaskDetail(task)
}

/** Task archivieren: in die _archive-Spalte verschieben und reaktiv aus der Liste entfernen. */
async function archiveTask(task: Task, event: Event): Promise<void> {
  event.stopPropagation()
  if (!state.project) return

  // _archive-Spalte suchen
  const archiveCol = state.project.columns.find((c: Column) => c.title === '_archive')
  if (!archiveCol) {
    toast.error('Archive column not found')
    return
  }

  // Optimistisch aus der Liste entfernen
  listTasks.value = listTasks.value.filter(t => t.id !== task.id)

  try {
    const order = state.project.tasks.filter((t: Task) => t.column_id === archiveCol.id).length
    await api.post(`/api/projects/${state.project._id}/tasks/${task.id}/move`, {
      column_id: archiveCol.id,
      order,
    })
    // State aktualisieren (Task aus project.tasks entfernen bzw. column_id aktualisieren)
    const stateTask = state.project.tasks.find((t: Task) => t.id === task.id)
    if (stateTask) {
      stateTask.column_id = archiveCol.id
      stateTask.order = order
    }
    toast.success(t('archive.archived', { title: task.title }))
  } catch (err) {
    console.error('[ListBoard] archive failed:', err)
    toast.error(String(err))
    // Rollback: Task wieder in die Liste einfügen
    refreshTasks()
  }
}

/** Neuen Task in der Liste erstellen. */
function addTask(): void {
  if (!listColumn.value) return
  // @ts-ignore
  if (typeof window.__openNewTaskModal === 'function') window.__openNewTaskModal(listColumn.value.id)
}

// Globale Bridge für SSE-Updates
// @ts-ignore
window.__kanbanRefresh = refreshTasks
// @ts-ignore
window.__kanbanToggleSearch = () => {} // no-op für List-Board
</script>

<template>
  <!-- List-Board: eine Spalte, kompaktes Layout, kein Spalten-Drag & Drop -->
  <div class="list-board w-full max-w-[720px] mx-auto py-2">
    <div
      v-if="listColumn"
      class="list-board-column bg-surface rounded-lg flex flex-col"
      :data-id="listColumn.id"
    >
      <!-- Task-Liste mit vertikalem Drag & Drop (nur innerhalb der einen Spalte) -->
      <VueDraggable
        v-model="listTasks"
        group="list-tasks"
        :animation="150"
        :delay="400"
        :delay-on-touch-only="true"
        :touch-start-threshold="5"
        :scroll="true"
        :scroll-sensitivity="100"
        :scroll-speed="15"
        :bubble-scroll="true"
        :force-fallback="true"
        :fallback-on-body="true"
        class="list-task-list flex flex-col gap-1.5 px-1 py-1"
        ghost-class="sortable-ghost"
        chosen-class="sortable-chosen"
        fallback-class="sortable-fallback"
        @update="(evt: any) => onSortUpdate(evt)"
      >
        <div
          v-for="task in listTasks"
          :key="task.id"
          class="list-item kanban-item cursor-grab active:cursor-grabbing bg-surface-2 border border-border rounded-md transition-[border-color,box-shadow] duration-150 select-none hover:border-accent-dim hover:shadow-[0_2px_12px_rgba(124,106,247,0.15)]"
          :data-task-id="task.id"
          @click="handleTaskClick(task, $event)"
        >
          <div
            class="group/task flex items-start gap-2 px-3 py-2 cursor-pointer"
            :class="{ 'outline-2 outline-accent -outline-offset-2 rounded-md': state.selectedTasks.has(task.id) }"
            :style="{ borderLeft: `3px solid ${workerBorderColor(task.worker)}` }"
          >
            <input
              type="checkbox"
              class="task-checkbox mt-0.5 flex-shrink-0"
              :data-task-id="task.id"
              :checked="state.selectedTasks.has(task.id)"
              @change="toggleTaskSelection(task.id, ($event.target as HTMLInputElement).checked)"
              @click.stop
            />
            <span v-if="task.task_type === 'epic'" class="inline-flex items-center justify-center font-mono text-[9px] font-bold w-[18px] h-[18px] rounded-sm flex-shrink-0 bg-badge-epic-bg text-badge-epic-text border border-badge-epic-border mt-0.5" title="Epic">E</span>
            <span v-else-if="task.task_type === 'job'" class="inline-flex items-center justify-center font-mono text-[9px] font-bold w-[18px] h-[18px] rounded-sm flex-shrink-0 bg-badge-job-bg text-badge-job-text border border-badge-job-border mt-0.5" title="Job">J</span>
            <div class="flex-1 min-w-0">
              <div class="flex items-start justify-between gap-1.5 mb-0.5">
                <div class="text-[13px] font-semibold text-text leading-snug">{{ task.title }}</div>
                <div class="flex items-center gap-1 flex-shrink-0">
                  <span v-if="isBlocked(task)" class="inline-flex items-center justify-center font-mono text-[9px] font-bold w-[18px] h-[18px] rounded-sm bg-badge-blocked-bg text-badge-blocked-text border border-badge-blocked-border" title="Blocked">B</span>
                  <span v-if="task.points" class="bg-accent-dim border border-accent rounded-[10px] text-accent font-mono text-[10px] font-semibold px-[7px] py-px">{{ task.points }}</span>
                  <button
                    class="task-archive-btn inline-flex items-center justify-center w-[22px] h-[22px] rounded-sm text-text-dim opacity-0 group-hover/task:opacity-100 hover:bg-surface hover:text-accent transition-all duration-150"
                    :title="t('archive.archiveTask')"
                    @click.stop="archiveTask(task, $event)"
                  >
                    <svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                      <polyline points="21 8 21 21 3 21 3 8"></polyline>
                      <rect x="1" y="3" width="22" height="5"></rect>
                      <line x1="10" y1="12" x2="14" y2="12"></line>
                    </svg>
                  </button>
                </div>
              </div>
              <div v-if="task.task_type === 'epic' && (task.subtask_ids || []).length" class="flex items-center gap-1.5 py-0.5">
                <span class="flex-1 h-1 bg-border rounded-sm overflow-hidden">
                  <span class="block h-full bg-success rounded-sm transition-all duration-200" :style="{ width: subtaskProgress(task).pct + '%' }"></span>
                </span>
                <span :class="['font-mono text-[10px] text-text-dim', { '!text-success': subtaskProgress(task).done === subtaskProgress(task).total }]">
                  {{ subtaskProgress(task).done }}/{{ subtaskProgress(task).total }}
                </span>
              </div>
              <div v-if="task.description" class="text-xs text-text-dim leading-snug">
                {{ ((p) => p.substring(0, 120) + (p.length > 120 ? '…' : ''))(stripMarkdown(task.description)) }}
              </div>
              <div class="flex items-center justify-between gap-1.5 flex-wrap mt-1">
                <div class="flex gap-1 flex-wrap">
                  <span v-for="label in (task.labels || [])" :key="label"
                    class="font-mono text-[10px] px-1.5 py-px rounded-sm border"
                    :style="{ background: labelColor(label).bg, borderColor: labelColor(label).border, color: labelColor(label).color }">{{ label }}</span>
                </div>
                <span v-if="task.worker" class="inline-flex items-center justify-center rounded-full font-mono text-[10px] h-5 w-5 uppercase bg-surface border border-border text-text-dim" :title="task.worker">{{ task.worker[0].toUpperCase() }}</span>
              </div>
            </div>
          </div>
        </div>
      </VueDraggable>

      <!-- Leere Liste Placeholder -->
      <div v-if="!listTasks.length" class="p-6 text-center opacity-50 text-sm">
        {{ t('board.noTasks') }}
      </div>

      <!-- Task hinzufügen -->
      <div class="px-3 py-2 border-t border-border">
        <button
          class="w-full bg-transparent border border-dashed border-border rounded-md text-text-dim text-xs py-1.5 cursor-pointer transition-all hover:border-accent hover:text-accent"
          @click="addTask"
        >+ {{ t('board.addTask') }}</button>
      </div>
    </div>

    <!-- Fallback: kein Projekt geladen -->
    <div v-else class="text-center text-text-dim opacity-50 py-12">
      {{ t('board.noTasks') }}
    </div>
  </div>
</template>
