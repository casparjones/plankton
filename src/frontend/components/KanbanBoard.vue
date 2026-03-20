<script setup lang="ts">
// Kanban-Board Komponente mit VueDraggablePlus für Drag&Drop.
// Ersetzt die bisherige jKanban-Implementierung.

import { ref, computed, watch, nextTick, onMounted, onUnmounted } from 'vue'
import { VueDraggable } from 'vue-draggable-plus'
import Sortable from 'sortablejs'
import type { Task, Column } from '../types'

import { state } from '../state'
import api from '../api'
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

/** Batch-Move: alle Moves in einem Request. */
function persistMoves(moves: { task_id: string; column_id: string; order: number }[]): void {
  if (!moves.length) return
  api.post(`/api/projects/${state.project!._id}/tasks/batch-move`, { moves })
    .then(() => {
      // Toast nur bei Spalten-Wechsel (nicht bei Reorder innerhalb einer Spalte)
      if (moves.length === 1) {
        const m = moves[0]
        const task = state.project?.tasks.find((t: Task) => t.id === m.task_id)
        const col = state.project?.columns.find((c: Column) => c.id === m.column_id)
        if (task && col && task.column_id !== m.column_id) {
          toast.success(`"${task.title}" → ${col.title}`)
        }
      }
    })
    .catch(err => {
      console.error('[DnD] ❌ batch-move failed:', err)
      toast.error('Verschieben fehlgeschlagen')
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

/** SortableJS @add: Task aus anderer Spalte hierhin verschoben. */
function onSortAdd(columnId: string, evt: any): void {
  const tasks = columnTasks.value[columnId] || []
  const task = tasks[evt.newIndex]
  if (!task) return
  // Warnung: blockierte Tasks können nicht auf Done verschoben werden.
  const doneCol = state.project?.columns.find((c: Column) => c.title === 'Done')
  if (doneCol && columnId === doneCol.id && isBlocked(task)) {
    const blockerNames = (task.blocked_by || [])
      .map(bid => state.project!.tasks.find((t: Task) => t.id === bid))
      .filter(t => t && t.column_id !== doneCol.id)
      .map(t => `"${t!.title}"`)
      .join(', ')
    toast.error(`Blockiert durch: ${blockerNames}`, {
      timeout: 5000,
    })
    // VueDraggable hat den Task ins neue Array verschoben.
    // column_id ist aber noch die alte Spalte → refreshColumnTasks baut korrekt neu.
    refreshColumnTasks()
    return
  }
  persistMoves([{ task_id: task.id, column_id: columnId, order: evt.newIndex }])
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

onMounted(() => {
  nextTick(() => {
    if (!columnsRef.value) return
    Sortable.create(columnsRef.value, {
      animation: 150,
      handle: '.kanban-board-header',
      draggable: '.kanban-column:not(.col-done)',
      ghostClass: 'column-dragging',
      onEnd: onColumnReorder,
    })
  })
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
  <div v-if="showSearch" class="board-search">
    <div class="search-bar">
      <input
        ref="searchInputRef"
        v-model="searchQuery"
        type="text"
        placeholder="Suche in Titel & Beschreibung… (Esc zum Schließen)"
        class="search-input"
        @keydown="onSearchKeydown"
      />
      <select v-model="filterLabel" class="search-select">
        <option value="">Alle Labels</option>
        <option v-for="l in allLabels" :key="l" :value="l">{{ l }}</option>
      </select>
      <select v-model="filterWorker" class="search-select">
        <option value="">Alle Worker</option>
        <option v-for="w in allWorkers" :key="w" :value="w">{{ w }}</option>
      </select>
      <button v-if="hasActiveFilter" class="search-clear" @click="clearFilters" title="Filter zurücksetzen">&#10005;</button>
      <button class="search-close" @click="toggleSearch" title="Suche schließen">Esc</button>
    </div>
  </div>
  <div ref="columnsRef" class="board-columns">
    <!-- Spalten-Container (Drag&Drop für Spaltenreihenfolge deaktiviert für Done) -->
    <div
      v-for="col in sortedColumns"
      :key="col.id"
      class="kanban-column"
      :class="{ 'col-done': col.title === 'Done' }"
      :data-id="col.id"
    >
      <!-- Spalten-Header -->
      <div class="kanban-board-header" :style="{ borderTop: `3px solid ${col.color}` }">
        <span class="col-title" :style="{ borderColor: col.color }">{{ col.title }}</span>
        <span class="col-count">{{ (columnTasks[col.id] || []).length }}</span>
        <div class="col-actions">
          <button class="col-add-btn" :data-col-id="col.id" title="Task hinzufügen" @click="addTask(col.id)">+</button>
          <button class="col-menu-btn" :data-col-id="col.id" title="Spalte verwalten" @click="openColMenu($event, col.id)">&#9776;</button>
        </div>
      </div>

      <!-- Task-Liste mit Drag&Drop -->
      <VueDraggable
        v-model="columnTasks[col.id]"
        group="tasks"
        :animation="150"
        class="kanban-drag"
        ghost-class="dragging"
        @start="onDragStart"
        @end="onDragEnd"
        @update="(evt: any) => onSortUpdate(col.id, evt)"
        @add="(evt: any) => onSortAdd(col.id, evt)"
      >
        <div
          v-for="task in (columnTasks[col.id] || [])"
          :key="task.id"
          class="kanban-item"
          @click="handleTaskClick(task, $event)"
        >
          <div
            class="task-inner"
            :class="{ 'task-selected': state.selectedTasks.has(task.id) }"
            :data-task-id="task.id"
            :style="{ borderLeft: `3px solid ${workerBorderColor(task.worker)}` }"
          >
            <div class="task-header-row">
              <input
                type="checkbox"
                class="task-checkbox"
                :data-task-id="task.id"
                :checked="state.selectedTasks.has(task.id)"
                @change="toggleTaskSelection(task.id, ($event.target as HTMLInputElement).checked)"
                @click.stop
              />
              <span v-if="task.task_type === 'epic'" class="type-badge type-epic" title="Epic">E</span>
              <span v-else-if="task.task_type === 'job'" class="type-badge type-job" title="Job">J</span>
              <div class="task-title">{{ task.title }}</div>
              <span v-if="isBlocked(task)" class="blocked-badge" title="Blocked">B</span>
              <span v-if="task.points" class="points-badge">{{ task.points }}</span>
            </div>
            <div v-if="task.task_type === 'epic' && (task.subtask_ids || []).length" class="subtask-progress">
              <span class="subtask-bar">
                <span class="subtask-fill" :style="{ width: subtaskProgress(task).pct + '%' }"></span>
              </span>
              <span :class="['subtask-count', { 'subtask-done': subtaskProgress(task).done === subtaskProgress(task).total }]">
                {{ subtaskProgress(task).done }}/{{ subtaskProgress(task).total }}
              </span>
            </div>
            <div v-if="task.description" class="task-desc">
              {{ task.description.substring(0, 80) }}{{ task.description.length > 80 ? '…' : '' }}
            </div>
            <div class="task-meta">
              <div class="task-labels">
                <span v-for="label in (task.labels || [])" :key="label" class="label"
                  :style="{ background: labelColor(label).bg, borderColor: labelColor(label).border, color: labelColor(label).color }">{{ label }}</span>
              </div>
              <div class="task-assignees">
                <span v-if="task.worker" class="avatar" :title="task.worker">{{ task.worker[0].toUpperCase() }}</span>
              </div>
            </div>
          </div>
        </div>
      </VueDraggable>

      <!-- Leere Spalte Placeholder -->
      <div v-if="!(columnTasks[col.id] || []).length" class="kanban-empty">
        Keine Tasks
      </div>
    </div>
  </div>
</template>

<style scoped>
.board-columns {
  display: flex;
  gap: 12px;
  overflow-x: auto;
  padding: 8px 0;
  min-height: 200px;
  align-items: flex-start;
}

.kanban-column {
  min-width: 280px;
  max-width: 320px;
  flex: 0 0 300px;
  background: var(--board-bg, #1a1a2e);
  border-radius: 8px;
  display: flex;
  flex-direction: column;
  max-height: calc(100vh - 140px);
}

.kanban-board-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border-radius: 8px 8px 0 0;
  flex-shrink: 0;
  cursor: grab;
}

.kanban-board-header:active {
  cursor: grabbing;
}

.col-done .kanban-board-header {
  cursor: default;
}

.column-dragging {
  opacity: 0.4;
}

.kanban-drag {
  min-height: 40px;
  padding: 4px 8px;
  flex: 1;
  min-height: 0;
  overflow-y: auto;
}

.kanban-item {
  cursor: grab;
  margin-bottom: 6px;
}

.kanban-item:active {
  cursor: grabbing;
}

.kanban-empty {
  padding: 16px;
  text-align: center;
  opacity: 0.5;
  font-size: 0.85rem;
}
</style>
