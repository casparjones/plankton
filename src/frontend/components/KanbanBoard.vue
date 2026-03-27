<script setup lang="ts">
// Kanban-Board Komponente mit VueDraggablePlus für Drag&Drop.
// Ersetzt die bisherige jKanban-Implementierung.

import { ref, computed, watch, nextTick, onMounted, onUnmounted } from 'vue'
import { VueDraggable } from 'vue-draggable-plus'
import Sortable from 'sortablejs'
import { marked } from 'marked'
import type { Task, Column, ProjectDoc } from '../types'

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

const toast = useToast()
import { updateBulkBar } from './bulk-actions'
// @ts-ignore
import { updateGitStatusIcon } from './git-settings'
import { escapeHtml, labelColor } from '../utils'

// Feste Farbzuweisung pro Worker (Hash → Farbpalette).
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

/** Prüft ob ein Task blockiert ist (mindestens ein Blocker ist nicht in Done). */
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

// ─── Suche & Filter ──────────────────────────────────────────
const searchQuery = ref('')
const filterLabel = ref('')
const filterWorker = ref('')
const searchInputRef = ref<HTMLInputElement | null>(null)
const showSearch = ref(false)

/** Alle einzigartigen Labels im Projekt. */
const allLabels = computed(() => {
  const set = new Set<string>()
  for (const t of (state.project?.tasks || [])) {
    for (const l of (t.labels || [])) set.add(l)
  }
  return [...set].sort()
})

/** Alle einzigartigen Worker im Projekt. */
const allWorkers = computed(() => {
  const set = new Set<string>()
  for (const t of (state.project?.tasks || [])) {
    if (t.worker) set.add(t.worker)
  }
  return [...set].sort()
})

const hasActiveFilter = computed(() => searchQuery.value || filterLabel.value || filterWorker.value)

/** Prüft ob ein Task den aktiven Filtern entspricht. */
function matchesFilter(task: Task): boolean {
  if (!hasActiveFilter.value) return true
  const q = searchQuery.value.toLowerCase()
  if (q && !task.title.toLowerCase().includes(q) && !task.description?.toLowerCase().includes(q)) return false
  if (filterLabel.value && !(task.labels || []).includes(filterLabel.value)) return false
  if (filterWorker.value && task.worker !== filterWorker.value) return false
  return true
}

function toggleSearch(): void {
  showSearch.value = !showSearch.value
  if (showSearch.value) {
    nextTick(() => searchInputRef.value?.focus())
  } else {
    clearFilters()
  }
}

function clearFilters(): void {
  searchQuery.value = ''
  filterLabel.value = ''
  filterWorker.value = ''
}

function onSearchKeydown(e: KeyboardEvent): void {
  if (e.key === 'Escape') {
    toggleSearch()
  }
}

// Ctrl+K / Cmd+K Shortcut
function handleGlobalKeydown(e: KeyboardEvent): void {
  if ((e.ctrlKey || e.metaKey) && (e.key === 'k' || e.key === 's')) {
    e.preventDefault()
    showSearch.value = true
    nextTick(() => searchInputRef.value?.focus())
  }
}
onMounted(() => document.addEventListener('keydown', handleGlobalKeydown))
onUnmounted(() => document.removeEventListener('keydown', handleGlobalKeydown))

/** Sichtbare Spalten, sortiert (Done immer zuletzt). */
const sortedColumns = computed<Column[]>(() => {
  console.log('[KanbanBoard] sortedColumns computed, state.project:', state.project?._id, 'columns:', state.project?.columns?.length)
  if (!state.project?.columns) return []
  const result = [...state.project.columns]
    .filter((c: Column) => !c.hidden)
    .sort((a: Column, b: Column) => {
      const aIsDone = a.title === 'Done'
      const bIsDone = b.title === 'Done'
      if (aIsDone && !bIsDone) return 1
      if (!aIsDone && bIsDone) return -1
      return a.order - b.order
    })
  console.log('[KanbanBoard] sortedColumns result:', result.length, result.map(c => c.title))
  return result
})

/** Tasks pro Spalte, sortiert nach order, gefiltert nach Suchkriterien. */
function tasksForColumn(columnId: string): Task[] {
  return (state.project?.tasks || [])
    .filter((t: Task) => t.column_id === columnId && matchesFilter(t))
    .sort((a: Task, b: Task) => a.order - b.order)
}

/** Reaktive Task-Listen pro Spalte für VueDraggable. */
const columnTasks = ref<Record<string, Task[]>>({})

/** Aktualisiert die Task-Listen aus dem State. */
function refreshColumnTasks(): void {
  console.log('[KanbanBoard] refreshColumnTasks called')
  console.log('[KanbanBoard] state.project:', state.project?._id, state.project?.title)
  console.log('[KanbanBoard] sortedColumns count:', sortedColumns.value.length)
  const result: Record<string, Task[]> = {}
  for (const col of sortedColumns.value) {
    result[col.id] = tasksForColumn(col.id)
    console.log('[KanbanBoard] column', col.title, '→', result[col.id].length, 'tasks')
  }
  columnTasks.value = result
  console.log('[KanbanBoard] columnTasks updated:', Object.keys(result).length, 'columns')
  // Apply glow animation to newly created/imported tasks.
  const glowId = (window as any).__newTaskGlowId
  const glowIds: string[] = (window as any).__newTaskGlowIds || []
  if (glowId) glowIds.push(glowId)
  delete (window as any).__newTaskGlowId
  delete (window as any).__newTaskGlowIds
  if (glowIds.length > 0) {
    nextTick(() => {
      let firstEl: HTMLElement | null = null
      for (const id of glowIds) {
        const el = document.querySelector(`.kanban-item[data-task-id="${id}"]`) as HTMLElement | null
        if (el) {
          el.classList.add('task-new-glow')
          setTimeout(() => el.classList.remove('task-new-glow'), 2500)
          if (!firstEl) firstEl = el
        }
      }
      if (firstEl) firstEl.scrollIntoView({ behavior: 'smooth', block: 'nearest' })
    })
  }
}

// Initialer Aufbau und bei Projekt-Änderungen aktualisieren.
watch(() => state.project, () => {
  nextTick(() => {
    refreshColumnTasks()
    updateGitStatusIcon()
    updateBulkBar()
  })
}, { deep: true, immediate: true })

// Filter-Änderungen aktualisieren das Board.
watch([searchQuery, filterLabel, filterWorker], () => {
  refreshColumnTasks()
})

/** Handler für Task-Drag-Start. */
function onDragStart(): void {
  console.log('[DnD] drag START')
  state.isDragging = true
}

/** Handler für Task-Drag-Ende. */
function onDragEnd(evt: any): void {
  console.log('[DnD] drag END', evt)
  setTimeout(() => { state.isDragging = false }, 50)
}

/** Batch-Move: alle Moves in einem Request. Lokalen State sofort aktualisieren. */
function persistMoves(moves: { task_id: string; column_id: string; order: number }[]): void {
  if (!moves.length) return
  // Snapshot für Rollback bei Fehler
  const snapshot: { task_id: string; column_id: string; order: number }[] = []
  // Lokalen State sofort aktualisieren (column_id + order) – optimistic update
  const movedTasks: { title: string; colTitle: string }[] = []
  for (const m of moves) {
    const task = state.project?.tasks.find((t: Task) => t.id === m.task_id)
    if (task) {
      snapshot.push({ task_id: task.id, column_id: task.column_id, order: task.order })
      const isColumnChange = task.column_id !== m.column_id
      task.column_id = m.column_id
      task.order = m.order
      if (isColumnChange) {
        const col = state.project?.columns.find((c: Column) => c.id === m.column_id)
        if (col) movedTasks.push({ title: task.title, colTitle: col.title })
      }
    }
  }
  api.post(`/api/projects/${state.project!._id}/tasks/batch-move`, { moves })
    .then(async () => {
      // Success toast only after server confirms
      for (const mt of movedTasks) {
        toast.success(`"${mt.title}" → ${mt.colTitle}`)
      }
      // Server-State nachladen um Logs und andere serverseitige Änderungen zu erhalten
      const p = await api.get<ProjectDoc>(`/api/projects/${state.project!._id}`)
      state.project = p
    })
    .catch(err => {
      console.error('[DnD] ❌ batch-move failed:', err)
      // Use i18n key from server error code if available
      if (err instanceof ApiError && err.code) {
        const i18nKey = `drag.${err.code}`
        toast.error(t(i18nKey, { details: err.message }), { timeout: 5000 })
      } else {
        toast.error(t('drag.moveFailed'), { timeout: 5000 })
      }
      // Rollback: optimistic update rückgängig machen
      for (const s of snapshot) {
        const task = state.project?.tasks.find((t: Task) => t.id === s.task_id)
        if (task) {
          task.column_id = s.column_id
          task.order = s.order
        }
      }
      refreshColumnTasks()
    })
}

/** SortableJS @update: Task innerhalb derselben Spalte verschoben. */
function onSortUpdate(columnId: string, evt: any): void {
  const tasks = columnTasks.value[columnId] || []
  const from = Math.min(evt.oldIndex, evt.newIndex)
  const to = Math.max(evt.oldIndex, evt.newIndex)
  const moves: { task_id: string; column_id: string; order: number }[] = []
  for (let i = from; i <= to; i++) {
    const task = tasks[i]
    if (task && task.order !== i) {
      task.order = i
      moves.push({ task_id: task.id, column_id: columnId, order: i })
    }
  }
  persistMoves(moves)
}

/** SortableJS :move – prüft VOR dem Move ob ein blockierter Task auf Done gezogen wird. */
function onMoveCheck(evt: any): boolean {
  const taskEl = evt.dragged
  const targetList = evt.to
  const targetColId = targetList?.closest('.kanban-column')?.dataset?.id
  if (!targetColId) return true
  const doneCol = state.project?.columns.find((c: Column) => c.title === 'Done')
  if (!doneCol || targetColId !== doneCol.id) return true
  const taskId = taskEl?.dataset?.taskId
  if (!taskId) return true
  const task = state.project?.tasks.find((t: Task) => t.id === taskId)
  if (!task || !isBlocked(task)) return true
  // Blockiert → Move verhindern und Toast zeigen
  const blockerNames = (task.blocked_by || [])
    .map(bid => state.project!.tasks.find((t: Task) => t.id === bid))
    .filter(t => t && t.column_id !== doneCol.id)
    .map(t => `"${t!.title}"`)
    .join(', ')
  toast.error(t('drag.blockedBy', { blockers: blockerNames }), { timeout: 5000 })
  return false
}

/** SortableJS @add: Task aus anderer Spalte hierhin verschoben. */
function onSortAdd(columnId: string, evt: any): void {
  const tasks = columnTasks.value[columnId] || []
  if (!tasks.length) return
  // Alle Tasks in der Zielspalte mit neuen order-Werten persistieren,
  // damit die Reihenfolge nach Server-Reload korrekt bleibt.
  const moves = tasks.map((t, i) => ({ task_id: t.id, column_id: columnId, order: i }))
  persistMoves(moves)
}

/** Spalten-Reihenfolge nach Drag persistieren. */
function onColumnReorder(evt: Sortable.SortableEvent): void {
  if (evt.oldIndex == null || evt.newIndex == null || evt.oldIndex === evt.newIndex) return
  // State aktualisieren
  const columns = state.project?.columns
  if (!columns) return
  // Neue Reihenfolge aus dem DOM lesen (data-id Attribute)
  const container = evt.from
  const orderedIds = Array.from(container.children).map(el => (el as HTMLElement).dataset.id)
  orderedIds.forEach((id, idx) => {
    const col = columns.find((c: Column) => c.id === id)
    if (col) col.order = idx
  })
  // Done-Spalte immer zuletzt
  const done = columns.find((c: Column) => c.title === 'Done')
  if (done) done.order = 9999
  // Projekt speichern
  api.put(`/api/projects/${state.project!._id}`, state.project)
    .catch(err => console.error('[DnD] column reorder failed:', err))
}

/** Template ref für den Spalten-Container. */
const columnsRef = ref<HTMLElement | null>(null)

let columnSortable: Sortable | null = null

function initColumnSortable(): void {
  if (columnSortable) columnSortable.destroy()
  nextTick(() => {
    if (!columnsRef.value) return
    columnSortable = Sortable.create(columnsRef.value, {
      animation: 150,
      handle: '.kanban-board-header',
      draggable: '.kanban-column:not(.col-done)',
      ghostClass: 'column-dragging',
      onEnd: onColumnReorder,
    })
  })
}

onMounted(initColumnSortable)

// Re-init Sortable wenn Projekt wechselt (neues Board, anderes Projekt)
watch(() => state.project?._id, () => {
  refreshColumnTasks()
  initColumnSortable()
})

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

  // Nutze globale Bridge-Funktion (registriert von TaskDetail.vue)
  // @ts-ignore
  if (typeof window.__openTaskDetail === 'function') window.__openTaskDetail(task)
}

/** Neuen Task in Spalte erstellen. */
function addTask(columnId: string): void {
  // Nutze globale Bridge-Funktion (registriert von TaskModal.vue)
  // @ts-ignore
  if (typeof window.__openNewTaskModal === 'function') window.__openNewTaskModal(columnId)
}

/** Spalten-Menü öffnen. */
function openColMenu(event: Event, columnId: string): void {
  event.stopPropagation()
  // @ts-ignore
  import('./column-modal').then(({ openColumnMenu }) => {
    openColumnMenu(event.target, columnId)
  })
}


// Globale Funktion für SSE-Updates und Legacy-Code (board.js Bridge).
// @ts-ignore
window.__kanbanRefresh = refreshColumnTasks
// @ts-ignore
window.__kanbanToggleSearch = toggleSearch
</script>

<template>
  <!-- Suchleiste -->
  <div v-if="showSearch" class="px-6 py-2 bg-surface border-b border-border">
    <div class="flex gap-2 items-center">
      <input
        ref="searchInputRef"
        v-model="searchQuery"
        type="text"
        :placeholder="t('board.searchPlaceholder')"
        class="flex-1 bg-surface-2 border border-border rounded-md text-text text-[13px] px-2.5 py-1.5 outline-none font-sans focus:border-accent placeholder:text-text-dim"
        @keydown="onSearchKeydown"
      />
      <select v-model="filterLabel" class="bg-surface-2 border border-border rounded-md text-text text-xs px-2 py-1.5 outline-none font-mono max-w-[160px] focus:border-accent">
        <option value="">{{ t('board.allLabels') }}</option>
        <option v-for="l in allLabels" :key="l" :value="l">{{ l }}</option>
      </select>
      <select v-model="filterWorker" class="bg-surface-2 border border-border rounded-md text-text text-xs px-2 py-1.5 outline-none font-mono max-w-[160px] focus:border-accent">
        <option value="">{{ t('board.allWorkers') }}</option>
        <option v-for="w in allWorkers" :key="w" :value="w">{{ w }}</option>
      </select>
      <button v-if="hasActiveFilter" class="bg-transparent border border-border rounded-md text-text-dim text-[11px] px-2 py-1 cursor-pointer font-mono hover:border-accent hover:text-accent" @click="clearFilters" :title="t('board.resetFilters')">&#10005;</button>
      <button class="bg-transparent border border-border rounded-md text-text-dim text-[11px] px-2 py-1 cursor-pointer font-mono hover:border-accent hover:text-accent" @click="toggleSearch" :title="t('board.closeSearch')">Esc</button>
    </div>
  </div>
  <div ref="columnsRef" class="flex gap-3 overflow-x-auto py-2 min-h-[200px] items-start">
    <div
      v-for="col in sortedColumns"
      :key="col.id"
      class="kanban-column min-w-[280px] max-w-[320px] flex-[0_0_300px] bg-surface rounded-lg flex flex-col max-h-[calc(100vh-140px)]"
      :class="{ 'col-done': col.title === 'Done' }"
      :data-id="col.id"
    >
      <!-- Spalten-Header -->
      <div class="kanban-board-header flex items-center gap-2 px-3 py-2.5 rounded-t-lg flex-shrink-0 cursor-grab active:cursor-grabbing" :class="{ '!cursor-default': col.title === 'Done' }" :style="{ borderTop: `3px solid ${col.color}` }">
        <span class="border-l-[3px] pl-2 text-text font-mono text-xs font-semibold uppercase tracking-wider" :style="{ borderColor: col.color }">{{ col.title }}</span>
        <span class="bg-surface-2 border border-border rounded-[10px] text-text-dim font-mono text-[10px] px-[7px] py-px ml-1.5">{{ (columnTasks[col.id] || []).length }}</span>
        <div class="flex gap-1 ml-auto">
          <button class="bg-transparent border border-border rounded text-text-dim cursor-pointer text-base h-[22px] leading-none px-1.5 transition-all hover:border-accent hover:text-accent" :data-col-id="col.id" :title="t('board.addTask')" @click="addTask(col.id)">+</button>
          <button class="bg-transparent border border-border rounded text-text-dim cursor-pointer text-xs h-[22px] leading-none px-[5px] transition-all hover:border-accent hover:text-accent" :data-col-id="col.id" :title="t('board.manageColumn')" @click="openColMenu($event, col.id)">&#9776;</button>
        </div>
      </div>

      <!-- Task-Liste mit Drag&Drop -->
      <VueDraggable
        v-model="columnTasks[col.id]"
        group="tasks"
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
        class="min-h-[40px] px-2 py-1 flex-1 overflow-y-auto"
        ghost-class="sortable-ghost"
        chosen-class="sortable-chosen"
        fallback-class="sortable-fallback"
        :move="onMoveCheck"
        @start="onDragStart"
        @end="onDragEnd"
        @update="(evt: any) => onSortUpdate(col.id, evt)"
        @add="(evt: any) => onSortAdd(col.id, evt)"
      >
        <div
          v-for="task in (columnTasks[col.id] || [])"
          :key="task.id"
          class="kanban-item cursor-grab active:cursor-grabbing mb-1.5 bg-surface-2 border border-border rounded-md transition-[border-color,box-shadow] duration-150 select-none hover:border-accent-dim hover:shadow-[0_2px_12px_rgba(124,106,247,0.15)]"
          :data-task-id="task.id"
          @click="handleTaskClick(task, $event)"
        >
          <div
            class="px-3 py-2.5 cursor-pointer"
            :class="{ 'outline-2 outline-accent -outline-offset-2 rounded-md': state.selectedTasks.has(task.id) }"
            :style="{ borderLeft: `3px solid ${workerBorderColor(task.worker)}` }"
          >
            <div class="flex items-start justify-between gap-1.5 mb-1">
              <input
                type="checkbox"
                class="task-checkbox"
                :data-task-id="task.id"
                :checked="state.selectedTasks.has(task.id)"
                @change="toggleTaskSelection(task.id, ($event.target as HTMLInputElement).checked)"
                @click.stop
              />
              <span v-if="task.task_type === 'epic'" class="inline-flex items-center justify-center font-mono text-[9px] font-bold w-[18px] h-[18px] rounded-sm flex-shrink-0 bg-badge-epic-bg text-badge-epic-text border border-badge-epic-border" title="Epic">E</span>
              <span v-else-if="task.task_type === 'job'" class="inline-flex items-center justify-center font-mono text-[9px] font-bold w-[18px] h-[18px] rounded-sm flex-shrink-0 bg-badge-job-bg text-badge-job-text border border-badge-job-border" title="Job">J</span>
              <div class="text-[13px] font-semibold text-text leading-snug flex-1">{{ task.title }}</div>
              <span v-if="isBlocked(task)" class="inline-flex items-center justify-center font-mono text-[9px] font-bold w-[18px] h-[18px] rounded-sm flex-shrink-0 bg-badge-blocked-bg text-badge-blocked-text border border-badge-blocked-border" title="Blocked">B</span>
              <span v-if="task.points" class="bg-accent-dim border border-accent rounded-[10px] text-accent font-mono text-[10px] font-semibold px-[7px] py-px flex-shrink-0">{{ task.points }}</span>
            </div>
            <div v-if="task.task_type === 'epic' && (task.subtask_ids || []).length" class="flex items-center gap-1.5 py-0.5">
              <span class="flex-1 h-1 bg-border rounded-sm overflow-hidden">
                <span class="block h-full bg-success rounded-sm transition-all duration-200" :style="{ width: subtaskProgress(task).pct + '%' }"></span>
              </span>
              <span :class="['font-mono text-[10px] text-text-dim', { '!text-success': subtaskProgress(task).done === subtaskProgress(task).total }]">
                {{ subtaskProgress(task).done }}/{{ subtaskProgress(task).total }}
              </span>
            </div>
            <div v-if="task.description" class="text-xs text-text-dim leading-snug mb-1.5">
              {{ ((p) => p.substring(0, 80) + (p.length > 80 ? '…' : ''))(stripMarkdown(task.description)) }}
            </div>
            <div class="flex items-center justify-between gap-1.5 flex-wrap">
              <div class="flex gap-1 flex-wrap">
                <span v-for="label in (task.labels || [])" :key="label"
                  class="font-mono text-[10px] px-1.5 py-px rounded-sm border"
                  :style="{ background: labelColor(label).bg, borderColor: labelColor(label).border, color: labelColor(label).color }">{{ label }}</span>
              </div>
              <div>
                <span v-if="task.worker" class="inline-flex items-center justify-center rounded-full font-mono text-[10px] h-5 w-5 uppercase bg-surface border border-border text-text-dim" :title="task.worker">{{ task.worker[0].toUpperCase() }}</span>
              </div>
            </div>
          </div>
        </div>
      </VueDraggable>

      <!-- Leere Spalte Placeholder -->
      <div v-if="!(columnTasks[col.id] || []).length" class="p-4 text-center opacity-50 text-sm">
        {{ t('board.noTasks') }}
      </div>
    </div>
  </div>
</template>

<style>
/* SortableJS drag styles – MUST be global (unscoped) because
   SortableJS clones elements to <body> outside Vue's scope */

/* Ghost = the placeholder left behind at the drop position */
.sortable-ghost {
  opacity: 0.3;
  background: var(--color-accent-dim) !important;
  border: 2px dashed var(--color-accent) !important;
  border-radius: var(--radius-md);
}
.sortable-ghost > * {
  visibility: hidden;
}

/* Chosen = the original element when picked up */
.sortable-chosen {
  opacity: 0.8;
  cursor: grabbing !important;
}

/* Fallback = the clone that follows the cursor (force-fallback: true)
   IMPORTANT: Do NOT set transform here — SortableJS uses transform: translate3d()
   to position the clone. Overriding it breaks movement. */
.sortable-fallback {
  opacity: 0.9 !important;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4), 0 0 0 2px var(--color-accent) !important;
  border-radius: var(--radius-md) !important;
  cursor: grabbing !important;
  z-index: 9999 !important;
  transition: none !important;
  pointer-events: none !important;
}

/* Column drag ghost */
.column-dragging {
  opacity: 0.4;
}

/* New task glow */
.kanban-item.task-new-glow {
  animation: task-glow 2s ease-out forwards;
}
</style>
