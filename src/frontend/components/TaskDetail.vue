<script setup lang="ts">
// Task-Detail: Nur-Lesen-Ansicht eines Tasks mit allen Informationen.
import { ref, computed } from 'vue'
import { marked } from 'marked'
import type { Task } from '../types'

import { state } from '../state'
import { columnName, formatDate, labelColor } from '../utils'
import { saveTask } from '../services/project-service'

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

const doneColId = computed(() => {
  return (state.project?.columns || []).find((c: { title: string }) => c.title === 'Done')?.id || ''
})

interface RelatedTask { id: string; title: string; done: boolean; taskType: string; colName: string }

function findTask(id: string): RelatedTask | null {
  const t = (state.project?.tasks || []).find((t: { id: string }) => t.id === id)
  if (!t) return null
  const col = (state.project?.columns || []).find((c: { id: string }) => c.id === t.column_id)
  return { id: t.id, title: t.title, done: t.column_id === doneColId.value, taskType: t.task_type || 'task', colName: col?.title || '–' }
}

/** Alle verknüpften Tickets gruppiert. */
const relatedTickets = computed(() => {
  if (!task.value) return []
  const groups: { label: string; icon: string; items: RelatedTask[] }[] = []

  // Parent Epic
  if (task.value.parent_id) {
    const p = findTask(task.value.parent_id)
    if (p) groups.push({ label: 'Epic', icon: '↑', items: [p] })
  }

  // Subtasks
  const subs = (task.value.subtask_ids || []).map(findTask).filter(Boolean) as RelatedTask[]
  if (subs.length) groups.push({ label: 'Subtasks', icon: '↳', items: subs })

  // Blockiert durch
  const by = (task.value.blocked_by || []).map(findTask).filter(Boolean) as RelatedTask[]
  if (by.length) groups.push({ label: 'Blockiert durch', icon: '⛔', items: by })

  // Blockiert
  const bl = (task.value.blocks || []).map(findTask).filter(Boolean) as RelatedTask[]
  if (bl.length) groups.push({ label: 'Blockiert', icon: '→', items: bl })

  return groups
})

const hasRelations = computed(() => relatedTickets.value.length > 0)

/** Öffnet ein verknüpftes Ticket im Detail-View. */
function openRelated(id: string): void {
  const t = (state.project?.tasks || []).find((t: Task) => t.id === id)
  if (t) open(t)
}
const newComment = ref('')

async function addComment(): Promise<void> {
  const text = newComment.value.trim()
  if (!text || !task.value) return
  const userName = state.currentUser?.display_name || state.currentUser?.username || 'anonymous'
  const now = new Date()
  const ts = `${String(now.getMonth()+1).padStart(2,'0')}-${String(now.getDate()).padStart(2,'0')} ${String(now.getHours()).padStart(2,'0')}:${String(now.getMinutes()).padStart(2,'0')}`
  task.value.comments.push({ ts, user: userName, msg: text } as any)
  newComment.value = ''
  await saveTask(task.value)
}

function open(t: Task): void {
  task.value = t
  state.detailTask = t
  isOpen.value = true
  // URL aktualisieren.
  if (t.id && state.project) {
    const pSlug = state.project.slug || state.project._id
    history.pushState({ project: pSlug, task: t.slug || t.id }, '', `/p/${pSlug}/t/${t.slug || t.id}`)
  }
}

function close(): void {
  isOpen.value = false
  task.value = null
  state.detailTask = null
  // URL zurück auf Projekt-Ebene.
  if (state.project) {
    history.pushState({ project: state.project.slug || state.project._id }, '', `/p/${state.project.slug || state.project._id}`)
  }
}

function editTask(): void {
  if (!task.value) return
  const t = task.value
  close()
  emit('edit', t)
}

const mcpLinkCopied = ref(false)

function copyMcpLink(): void {
  if (!task.value || !state.project) return
  const t = task.value
  const p = state.project
  const url = window.location.origin
  const col = (p.columns || []).find((c: { id: string }) => c.id === t.column_id)
  const colName = col?.title || '–'

  const prompt = [
    `Plankton-Ticket: ${url}/p/${p.slug || p._id}/t/${t.slug || t.id}`,
    `Projekt: "${p.title}" | ${t.task_type || 'task'}: "${t.title}" [${colName}]`,
    '',
    `Lade das Ticket mit dem plankton skill:`,
    `curl -s -X POST ${url}/mcp \\`,
    `  -H "Content-Type: application/json" \\`,
    `  -H "Authorization: Bearer $PLANKTON_TOKEN" \\`,
    `  -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"get_project","arguments":{"id":"${p._id}"}},"id":1}'`,
    '',
    `Task-ID: ${t.id}`,
    t.description ? `Beschreibung: ${t.description.substring(0, 200)}` : '',
    (t.labels || []).length ? `Labels: ${t.labels.join(', ')}` : '',
    t.worker ? `Worker: ${t.worker}` : '',
    t.points ? `Points: ${t.points}` : '',
  ].filter(Boolean).join('\n')

  // Clipboard API mit Fallback auf execCommand
  const doCopy = () => {
    mcpLinkCopied.value = true
    setTimeout(() => { mcpLinkCopied.value = false }, 2000)
  }
  if (navigator.clipboard?.writeText) {
    navigator.clipboard.writeText(prompt).then(doCopy).catch(() => {
      // Fallback: textarea + execCommand
      const ta = document.createElement('textarea')
      ta.value = prompt
      ta.style.position = 'fixed'
      ta.style.opacity = '0'
      document.body.appendChild(ta)
      ta.select()
      document.execCommand('copy')
      document.body.removeChild(ta)
      doCopy()
    })
  } else {
    const ta = document.createElement('textarea')
    ta.value = prompt
    ta.style.position = 'fixed'
    ta.style.opacity = '0'
    document.body.appendChild(ta)
    ta.select()
    document.execCommand('copy')
    document.body.removeChild(ta)
    doCopy()
  }
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
        <span class="modal-heading">{{ task?.task_type === 'epic' ? 'Epic' : task?.task_type === 'job' ? 'Job' : 'Task' }}</span>
        <button class="modal-close" @click="close">&#10005;</button>
      </div>
      <div class="detail-title">{{ task?.title }}</div>
      <div v-if="columnInfo" class="detail-column-info">
        <span class="label" :style="{ background: columnInfo.color + '22', borderColor: columnInfo.color, color: columnInfo.color }">{{ columnInfo.title }}</span>
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
                <span v-for="label in task!.labels" :key="label" class="label"
                  :style="{ background: labelColor(label).bg, borderColor: labelColor(label).border, color: labelColor(label).color }">{{ label }}</span>
              </template>
              <span v-else>–</span>
            </div>
          </div>
          <div v-if="hasRelations" class="detail-section">
            <span class="detail-section-title">Verknüpfte Tickets</span>
            <div class="related-tickets">
              <div v-for="group in relatedTickets" :key="group.label" class="related-group">
                <div class="related-group-label">{{ group.icon }} {{ group.label }}</div>
                <div v-for="item in group.items" :key="item.id" class="related-item" @click="openRelated(item.id)">
                  <span :class="['related-check', { done: item.done }]">{{ item.done ? '✓' : '○' }}</span>
                  <span v-if="item.taskType !== 'task'" class="related-type">{{ item.taskType === 'epic' ? 'E' : 'J' }}</span>
                  <span class="related-title">{{ item.title }}</span>
                  <span class="related-col">{{ item.colName }}</span>
                </div>
              </div>
            </div>
          </div>
          <div class="detail-section">
            <span class="detail-section-title">Kommentare</span>
            <div class="detail-list detail-comments-list">
              <template v-if="comments.length">
                <div v-for="(c, i) in comments" :key="i" class="detail-list-item log-entry">
                  <template v-if="typeof c === 'object' && c !== null">
                    <span class="log-ts">{{ c.ts }}</span>
                    <span class="log-user">{{ c.user }}</span>
                    <span class="log-msg markdown-body" v-html="renderMarkdown(c.msg)"></span>
                  </template>
                  <template v-else>
                    <span class="log-msg markdown-body" v-html="renderMarkdown(String(c))"></span>
                  </template>
                </div>
              </template>
              <div v-else class="detail-list-empty">Keine Kommentare</div>
            </div>
            <div class="detail-comment-input">
              <textarea
                v-model="newComment"
                placeholder="Kommentar schreiben…"
                rows="2"
                @keydown.ctrl.enter="addComment"
                @keydown.meta.enter="addComment"
              ></textarea>
              <button class="btn-primary btn-sm" @click="addComment" :disabled="!newComment.trim()">Senden</button>
            </div>
          </div>
        </div>
        <div class="detail-col-side">
          <div class="detail-side-actions">
            <button class="btn-mcp" @click="copyMcpLink" :title="mcpLinkCopied ? 'Kopiert!' : 'MCP-Link für Claude Code kopieren'">
              {{ mcpLinkCopied ? '✓ Kopiert' : 'MCP Link' }}
            </button>
            <button class="btn-primary" @click="editTask">Bearbeiten</button>
          </div>
          <div class="detail-section">
            <span class="detail-section-title">Details</span>
            <div class="detail-info-grid">
              <div class="detail-info-item">
                <span class="detail-info-item-label">Typ</span>
                <span class="detail-info-item-value">{{ task?.task_type || 'task' }}</span>
              </div>
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
            <span class="detail-section-title">Logs</span>
            <div class="detail-list">
              <template v-if="logs.length">
                <div v-for="(l, i) in logs" :key="i" class="detail-list-item log-entry">
                  <template v-if="typeof l === 'object'">
                    <span class="log-ts">{{ l.ts }}</span>
                    <span class="log-msg">{{ l.msg }}</span>
                    <span class="log-user">{{ l.user }}</span>
                  </template>
                  <template v-else>{{ l }}</template>
                </div>
              </template>
              <div v-else class="detail-list-empty">Keine Logs</div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
