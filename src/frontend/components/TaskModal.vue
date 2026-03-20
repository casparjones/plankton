<script setup lang="ts">
// Task-Modal: Erstellen und Bearbeiten von Tasks.
import { ref, computed, watch } from 'vue'
import type { Task } from '../types'

import { state } from '../state'
import { columnName, formatDate } from '../utils'
// @ts-ignore
import { saveTask, createTaskViaApi, deleteTask } from '../services/project-service'
import { toastConfirm } from '../toast'

const isOpen = ref(false)
const isNew = ref(false)
const editingTask = ref<Task | null>(null)

// Formular-Felder
const title = ref('')
const description = ref('')
const labels = ref('')
const points = ref(0)
const worker = ref('')
const newComment = ref('')
const taskType = ref('task')
const parentId = ref('')
const blockedBy = ref<string[]>([])
const blockedBySearch = ref('')
const blockedByOpen = ref(false)
const filteredBlockerTasks = computed(() => {
  const q = blockedBySearch.value.toLowerCase()
  return otherTasks.value.filter(t => !blockedBy.value.includes(t.id) && (!q || t.title.toLowerCase().includes(q)))
})
const selectedBlockers = computed(() =>
  blockedBy.value.map(id => otherTasks.value.find(t => t.id === id)).filter(Boolean) as Task[]
)
function addBlocker(id: string) {
  if (!blockedBy.value.includes(id)) blockedBy.value.push(id)
  blockedBySearch.value = ''
  blockedByOpen.value = false
}
function removeBlocker(id: string) {
  blockedBy.value = blockedBy.value.filter(v => v !== id)
}

const logs = computed(() => [...(editingTask.value?.logs || [])].reverse())
function formatLog(entry: string | { ts: string; user: string; msg: string }): string {
  if (typeof entry === 'string') return entry
  return `${entry.ts} ${entry.msg} [${entry.user}]`
}
function logParts(entry: string | { ts: string; user: string; msg: string }): { ts: string; msg: string; user: string } {
  if (typeof entry === 'string') return { ts: '', msg: entry, user: '' }
  return entry
}
const epics = computed(() =>
  (state.project?.tasks || []).filter((t: Task) => t.task_type === 'epic' && t.id !== editingTask.value?.id)
)
const otherTasks = computed(() =>
  (state.project?.tasks || []).filter((t: Task) => t.id !== editingTask.value?.id)
)
const comments = ref<string[]>([])
const createdAt = computed(() => formatDate(editingTask.value?.created_at))
const updatedAt = computed(() => formatDate(editingTask.value?.updated_at))
const previousRow = computed(() => columnName(editingTask.value?.previous_row))

/** Öffnet das Modal für einen neuen Task in der angegebenen Spalte. */
function openNew(columnId: string): void {
  const task: Task = {
    id: '',
    slug: '',
    title: '',
    description: '',
    column_id: columnId,
    column_slug: '',
    previous_row: '',
    assignee_ids: [],
    labels: [],
    order: (state.project?.tasks || []).filter((t: Task) => t.column_id === columnId).length,
    points: 0,
    worker: '',
    creator: '',
    logs: [],
    comments: [],
    created_at: '',
    updated_at: '',
    task_type: 'task',
    blocks: [],
    blocked_by: [],
    parent_id: '',
    subtask_ids: [],
  }
  openModal(task, true)
}

/** Öffnet das Modal zum Bearbeiten eines bestehenden Tasks. */
function openEdit(task: Task): void {
  openModal(task, false)
}

function openModal(task: Task, newTask: boolean): void {
  editingTask.value = { ...task, logs: [...(task.logs || [])], comments: [...(task.comments || [])] }
  // Auch den Legacy-State aktualisieren damit Legacy-Code kompatibel bleibt
  state.editingTask = editingTask.value
  state.isNewTask = newTask

  isNew.value = newTask
  title.value = task.title
  description.value = task.description || ''
  labels.value = (task.labels || []).join(', ')
  points.value = task.points || 0
  worker.value = task.worker || (newTask && state.currentUser ? state.currentUser.display_name : '')
  taskType.value = task.task_type || 'task'
  parentId.value = task.parent_id || ''
  blockedBy.value = [...(task.blocked_by || [])]
  comments.value = [...(task.comments || [])]
  newComment.value = ''
  isOpen.value = true

  // URL aktualisieren (nur bei existierenden Tasks).
  if (!newTask && task.id && state.project) {
    history.pushState({ project: state.project.slug || state.project._id, task: task.slug || task.id }, '', `/p/${state.project.slug || state.project._id}/t/${task.slug || task.id}`)
  }

  if (newTask) {
    setTimeout(() => {
      const el = document.querySelector('.task-modal-title-input') as HTMLInputElement
      el?.focus()
    }, 50)
  }
}

function close(): void {
  isOpen.value = false
  editingTask.value = null
  state.editingTask = null
  state.isNewTask = false
  // URL zurück auf Projekt-Ebene.
  if (state.project) {
    history.pushState({ project: state.project.slug || state.project._id }, '', `/p/${state.project.slug || state.project._id}`)
  }
}

async function save(): Promise<void> {
  if (!editingTask.value) return
  const oldBlockedBy = editingTask.value.blocked_by || []
  const newBlockedBy = blockedBy.value
  const task = {
    ...editingTask.value,
    title: title.value || 'Untitled',
    description: description.value,
    labels: labels.value.split(',').map(s => s.trim()).filter(Boolean),
    points: points.value || 0,
    worker: worker.value.trim(),
    task_type: taskType.value,
    parent_id: parentId.value,
    blocked_by: newBlockedBy,
    comments: comments.value,
  }
  if (isNew.value) {
    await createTaskViaApi(task)
  } else {
    await saveTask(task)
    // Bidirektionale Synchronisation: blocks-Feld der betroffenen Tasks aktualisieren
    const taskId = editingTask.value.id
    const added = newBlockedBy.filter(id => !oldBlockedBy.includes(id))
    const removed = oldBlockedBy.filter(id => !newBlockedBy.includes(id))
    const allTasks = state.project?.tasks || []
    for (const blockerId of added) {
      const blocker = allTasks.find((t: Task) => t.id === blockerId)
      if (blocker && !blocker.blocks.includes(taskId)) {
        await saveTask({ ...blocker, blocks: [...blocker.blocks, taskId] }, true)
      }
    }
    for (const blockerId of removed) {
      const blocker = allTasks.find((t: Task) => t.id === blockerId)
      if (blocker && blocker.blocks.includes(taskId)) {
        await saveTask({ ...blocker, blocks: blocker.blocks.filter(id => id !== taskId) }, true)
      }
    }
  }
  close()
}

async function handleDelete(): Promise<void> {
  if (!editingTask.value) return
  const ok = await toastConfirm(`Task "${editingTask.value.title}" löschen?`)
  if (ok) {
    deleteTask(editingTask.value.id)
    close()
  }
}

function addComment(): void {
  const text = newComment.value.trim()
  if (!text || !editingTask.value) return
  comments.value.push(text)
  editingTask.value.comments = comments.value
  newComment.value = ''
}

function onOverlayClick(e: Event): void {
  if ((e.target as HTMLElement).classList.contains('modal-overlay')) close()
}

// Globale Funktionen für Legacy-Kompatibilität (KanbanBoard.vue, AppLayout.vue)
// @ts-ignore
window.__openNewTaskModal = openNew
// @ts-ignore
window.__openTaskModal = openEdit
// @ts-ignore
window.__closeTaskModal = close

defineExpose({ openNew, openEdit, close })
</script>

<template>
  <div v-if="isOpen" class="modal-overlay open" @click="onOverlayClick">
    <div class="modal modal-wide">
      <div class="modal-header">
        <span class="modal-heading">{{ isNew ? 'Neuer Task' : 'Task bearbeiten' }}</span>
        <button class="modal-close" @click="close">&#10005;</button>
      </div>
      <div class="modal-grid">
        <div class="modal-col-main">
          <label>Titel
            <input v-model="title" type="text" class="task-modal-title-input" />
          </label>
          <label>Beschreibung
            <textarea v-model="description" rows="14"></textarea>
          </label>
          <label>Labels <small>(kommagetrennt)</small>
            <input v-model="labels" type="text" />
          </label>
          <div class="modal-section">
            <span class="modal-section-title">Kommentare</span>
            <div class="modal-list">
              <template v-if="comments.length">
                <div v-for="(c, i) in comments" :key="i" class="modal-list-item">{{ c }}</div>
              </template>
              <div v-else class="modal-list-empty">Keine Kommentare</div>
            </div>
            <div class="comment-input-row">
              <input v-model="newComment" type="text" placeholder="Kommentar schreiben…" @keydown.enter="addComment" />
              <button class="btn-small" @click="addComment">+</button>
            </div>
          </div>
        </div>
        <div v-if="!isNew" class="modal-col-side">
          <label>Typ
            <select v-model="taskType">
              <option value="task">Task</option>
              <option value="epic">Epic</option>
              <option value="job">Job</option>
            </select>
          </label>
          <label>Parent Epic
            <select v-model="parentId">
              <option value="">–</option>
              <option v-for="e in epics" :key="e.id" :value="e.id">{{ e.title }}</option>
            </select>
          </label>
          <label>Points <small>(0–100)</small>
            <input v-model.number="points" type="number" min="0" max="100" />
          </label>
          <label>Worker
            <input v-model="worker" type="text" />
          </label>
          <div class="modal-info">
            <span class="modal-info-label">Erstellt</span>
            <span class="modal-info-value">{{ createdAt }}</span>
          </div>
          <div class="modal-info">
            <span class="modal-info-label">Geändert</span>
            <span class="modal-info-value">{{ updatedAt }}</span>
          </div>
          <div class="modal-info">
            <span class="modal-info-label">Vorherige Spalte</span>
            <span class="modal-info-value">{{ previousRow }}</span>
          </div>
          <div class="modal-section" v-if="otherTasks.length">
            <span class="modal-section-title">Blockiert durch</span>
            <div class="multiselect">
              <div class="multiselect-tags" v-if="selectedBlockers.length">
                <span v-for="t in selectedBlockers" :key="t.id" class="multiselect-tag">
                  {{ t.title }}
                  <button type="button" class="multiselect-tag-remove" @click="removeBlocker(t.id)">&times;</button>
                </span>
              </div>
              <div class="multiselect-input-wrap">
                <input
                  v-model="blockedBySearch"
                  type="text"
                  class="multiselect-input"
                  placeholder="Task suchen…"
                  @focus="blockedByOpen = true"
                  @blur="setTimeout(() => blockedByOpen = false, 150)"
                />
              </div>
              <div v-if="blockedByOpen && filteredBlockerTasks.length" class="multiselect-dropdown">
                <div
                  v-for="t in filteredBlockerTasks"
                  :key="t.id"
                  class="multiselect-option"
                  @mousedown.prevent="addBlocker(t.id)"
                >{{ t.title }}</div>
              </div>
            </div>
          </div>
          <div class="modal-section">
            <span class="modal-section-title">Logs</span>
            <div class="modal-list modal-list-small">
              <template v-if="logs.length">
                <div v-for="(l, i) in logs" :key="i" class="modal-list-item log-entry">
                  <template v-if="typeof l === 'object'">
                    <span class="log-ts">{{ l.ts }}</span>
                    <span class="log-msg">{{ l.msg }}</span>
                    <span class="log-user">{{ l.user }}</span>
                  </template>
                  <template v-else>{{ l }}</template>
                </div>
              </template>
              <div v-else class="modal-list-empty">Keine Logs</div>
            </div>
          </div>
        </div>
        <div v-else class="modal-col-side">
          <label>Typ
            <select v-model="taskType">
              <option value="task">Task</option>
              <option value="epic">Epic</option>
              <option value="job">Job</option>
            </select>
          </label>
          <label>Parent Epic
            <select v-model="parentId">
              <option value="">–</option>
              <option v-for="e in epics" :key="e.id" :value="e.id">{{ e.title }}</option>
            </select>
          </label>
          <label>Points <small>(0–100)</small>
            <input v-model.number="points" type="number" min="0" max="100" />
          </label>
          <label>Worker
            <input v-model="worker" type="text" />
          </label>
        </div>
      </div>
      <div class="modal-actions">
        <button class="btn-primary" @click="save">Speichern</button>
        <button v-if="!isNew" class="btn-danger" @click="handleDelete">Löschen</button>
      </div>
    </div>
  </div>
</template>
