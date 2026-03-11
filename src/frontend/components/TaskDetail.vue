<script setup lang="ts">
// Task-Detail: Nur-Lesen-Ansicht eines Tasks mit allen Informationen.
import { ref, computed } from 'vue'
import { marked } from 'marked'
import type { Task } from '../types'

import { state } from '../state'
import { columnName, formatDate } from '../utils'

// Marked Optionen: keine async, Zeilenumbrüche als <br>
marked.setOptions({ async: false, breaks: true })

/** Rendert Markdown zu sanitized HTML. */
function renderMarkdown(text: string | undefined): string {
  if (!text) return '–'
  return marked.parse(text) as string
}

const isOpen = ref(false)
const task = ref<Task | null>(null)

const emit = defineEmits<{
  (e: 'edit', task: Task): void
}>()

const columnInfo = computed(() => {
  if (!task.value) return null
  const col = (state.project?.columns || []).find((c: { id: string }) => c.id === task.value!.column_id)
  return col ? { title: col.title, color: col.color } : null
})

const logs = computed(() => [...(task.value?.logs || [])].reverse())
const comments = computed(() => task.value?.comments || [])

function open(t: Task): void {
  task.value = t
  state.detailTask = t
  isOpen.value = true
}

function close(): void {
  isOpen.value = false
  task.value = null
  state.detailTask = null
}

function editTask(): void {
  if (!task.value) return
  const t = task.value
  close()
  emit('edit', t)
}

function onOverlayClick(e: Event): void {
  if ((e.target as HTMLElement).classList.contains('modal-overlay')) close()
}

// Globale Funktionen für Legacy-Kompatibilität
// @ts-ignore
window.__openTaskDetail = open
// @ts-ignore
window.__closeTaskDetail = close

defineExpose({ open, close })
</script>

<template>
  <div v-if="isOpen" class="modal-overlay open" @click="onOverlayClick">
    <div class="modal modal-detail">
      <div class="modal-header">
        <span class="modal-heading">Task</span>
        <button class="modal-close" @click="close">&#10005;</button>
      </div>
      <div class="detail-title">{{ task?.title }}</div>
      <div v-if="columnInfo" class="detail-column-info">
        <span class="column-badge" :style="{ backgroundColor: columnInfo.color }">{{ columnInfo.title }}</span>
      </div>
      <div class="detail-grid">
        <div class="detail-col-main">
          <div class="detail-section">
            <span class="detail-section-title">Beschreibung</span>
            <div class="detail-description markdown-body" v-html="renderMarkdown(task?.description)"></div>
          </div>
          <div class="detail-section">
            <span class="detail-section-title">Labels</span>
            <div class="detail-labels">
              <template v-if="(task?.labels || []).length">
                <span v-for="label in task!.labels" :key="label" class="label">{{ label }}</span>
              </template>
              <span v-else>–</span>
            </div>
          </div>
          <div class="detail-section">
            <span class="detail-section-title">Kommentare</span>
            <div class="detail-list">
              <template v-if="comments.length">
                <div v-for="(c, i) in comments" :key="i" class="detail-list-item markdown-body" v-html="renderMarkdown(c)"></div>
              </template>
              <div v-else class="detail-list-empty">Keine Kommentare</div>
            </div>
          </div>
        </div>
        <div class="detail-col-side">
          <div class="detail-section">
            <span class="detail-section-title">Details</span>
            <div class="detail-info-grid">
              <div class="detail-info-item">
                <span class="detail-info-item-label">Points</span>
                <span class="detail-info-item-value">{{ task?.points || '–' }}</span>
              </div>
              <div class="detail-info-item">
                <span class="detail-info-item-label">Worker</span>
                <span class="detail-info-item-value">{{ task?.worker || '–' }}</span>
              </div>
              <div class="detail-info-item">
                <span class="detail-info-item-label">Erstellt</span>
                <span class="detail-info-item-value">{{ formatDate(task?.created_at) }}</span>
              </div>
              <div class="detail-info-item">
                <span class="detail-info-item-label">Geändert</span>
                <span class="detail-info-item-value">{{ formatDate(task?.updated_at) }}</span>
              </div>
            </div>
          </div>
          <div class="detail-section">
            <span class="detail-section-title">Vorherige Spalte</span>
            <div>{{ columnName(task?.previous_row) }}</div>
          </div>
          <div class="detail-section">
            <span class="detail-section-title">Logs</span>
            <div class="detail-list">
              <template v-if="logs.length">
                <div v-for="(l, i) in logs" :key="i" class="detail-list-item">{{ l }}</div>
              </template>
              <div v-else class="detail-list-empty">Keine Logs</div>
            </div>
          </div>
        </div>
      </div>
      <div class="modal-actions">
        <button class="btn-primary" @click="editTask">Bearbeiten</button>
      </div>
    </div>
  </div>
</template>
