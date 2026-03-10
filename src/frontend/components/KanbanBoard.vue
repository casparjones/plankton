<script setup lang="ts">
// Kanban-Board Komponente mit VueDraggablePlus für Drag&Drop.
// Ersetzt die bisherige jKanban-Implementierung.

import { ref, computed, watch, nextTick } from 'vue'
import { VueDraggable } from 'vue-draggable-plus'
import type { Task, Column } from '../types'

import { state } from '../state'
// @ts-ignore
import { moveTask } from '../services/project-service'
import { updateBulkBar } from './bulk-actions'
// @ts-ignore
import { updateGitStatusIcon } from './git-settings'
import { escapeHtml } from '../utils'

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

/** Tasks pro Spalte, sortiert nach order. */
function tasksForColumn(columnId: string): Task[] {
  return (state.project?.tasks || [])
    .filter((t: Task) => t.column_id === columnId)
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
console.log('[KanbanBoard] Setting up watcher on state.project, current state:', typeof state, 'project:', state.project)
console.log('[KanbanBoard] state object keys:', Object.keys(state))
watch(() => state.project, (newVal, oldVal) => {
  console.log('[KanbanBoard] watch triggered! old:', oldVal?._id, '→ new:', newVal?._id, newVal?.title)
  nextTick(() => {
    refreshColumnTasks()
    updateGitStatusIcon()
    updateBulkBar()
  })
}, { deep: true, immediate: true })

/** Handler für Task-Drag-Start. */
function onDragStart(): void {
  state.isDragging = true
}

/** Handler für Task-Drag-Ende. */
function onDragEnd(): void {
  setTimeout(() => { state.isDragging = false }, 50)
}

/** Handler wenn ein Task in eine Spalte gedroppt wird. */
function onTaskChange(columnId: string, evt: { moved?: { element: Task; newIndex: number }; added?: { element: Task; newIndex: number } }): void {
  if (evt.added) {
    // Task wurde in eine andere Spalte verschoben
    moveTask(evt.added.element.id, columnId, evt.added.newIndex, true)
  } else if (evt.moved) {
    // Task wurde innerhalb derselben Spalte verschoben
    moveTask(evt.moved.element.id, columnId, evt.moved.newIndex, true)
  }
}

/** Handler für Spalten-Reihenfolge nach Drag. */
function onColumnReorder(): void {
  // Spalten-Order im Backend aktualisieren
  const visibleColumns = sortedColumns.value
  // @ts-ignore – Legacy-Import
  import('../services/project-service').then(({ saveTask: _unused, ...mod }) => {
    // Direkt die Spaltenorder über die API speichern
    const columns = state.project?.columns
    if (!columns) return
    visibleColumns.forEach((col: Column, idx: number) => {
      const origCol = columns.find((c: Column) => c.id === col.id)
      if (origCol) origCol.order = idx
    })
    // Projekt speichern
    // @ts-ignore
    import('../api').then(({ default: api }) => {
      api.put(`/api/projects/${state.project._id}`, state.project)
    })
  })
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

/** Erzeugt einen Change-Handler für eine bestimmte Spalte. */
function makeChangeHandler(columnId: string) {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  return (evt: any) => onTaskChange(columnId, evt)
}

// Globale Funktion für SSE-Updates und Legacy-Code (board.js Bridge).
// @ts-ignore
window.__kanbanRefresh = refreshColumnTasks
</script>

<template>
  <div class="board-columns">
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
        @change="makeChangeHandler(col.id)"
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
              <div class="task-title">{{ task.title }}</div>
              <span v-if="task.points" class="points-badge">{{ task.points }}</span>
            </div>
            <div v-if="task.description" class="task-desc">
              {{ task.description.substring(0, 80) }}{{ task.description.length > 80 ? '…' : '' }}
            </div>
            <div class="task-meta">
              <div class="task-labels">
                <span v-for="label in (task.labels || [])" :key="label" class="label">{{ label }}</span>
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
}

.kanban-board-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border-radius: 8px 8px 0 0;
}

.kanban-drag {
  min-height: 40px;
  padding: 4px 8px;
  flex: 1;
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
