<script setup lang="ts">
// Task-Modal: Erstellen und Bearbeiten von Tasks.
import { ref, computed, watch } from 'vue'
import type { Task } from '../types'

import { state } from '../state'
import { columnName, formatDate } from '../utils'
// @ts-ignore
import { saveTask, createTaskViaApi, deleteTask } from '../services/project-service'

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

const logs = computed(() => [...(editingTask.value?.logs || [])].reverse())
const comments = ref<string[]>([])
const createdAt = computed(() => formatDate(editingTask.value?.created_at))
const updatedAt = computed(() => formatDate(editingTask.value?.updated_at))
const previousRow = computed(() => columnName(editingTask.value?.previous_row))

/** Öffnet das Modal für einen neuen Task in der angegebenen Spalte. */
function openNew(columnId: string): void {
  const task: Task = {
    id: '',
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
  comments.value = [...(task.comments || [])]
  newComment.value = ''
  isOpen.value = true

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
}

async function save(): Promise<void> {
  if (!editingTask.value) return
  const task = {
    ...editingTask.value,
    title: title.value || 'Untitled',
    description: description.value,
    labels: labels.value.split(',').map(s => s.trim()).filter(Boolean),
    points: points.value || 0,
    worker: worker.value.trim(),
    comments: comments.value,
  }
  if (isNew.value) {
    await createTaskViaApi(task)
  } else {
    await saveTask(task)
  }
  close()
}

function handleDelete(): void {
  if (!editingTask.value) return
  if (confirm(`Task "${editingTask.value.title}" wirklich löschen?`)) {
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
            <textarea v-model="description" rows="8"></textarea>
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
          <div class="modal-section">
            <span class="modal-section-title">Logs</span>
            <div class="modal-list modal-list-small">
              <template v-if="logs.length">
                <div v-for="(l, i) in logs" :key="i" class="modal-list-item">{{ l }}</div>
              </template>
              <div v-else class="modal-list-empty">Keine Logs</div>
            </div>
          </div>
        </div>
        <div v-else class="modal-col-side">
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
