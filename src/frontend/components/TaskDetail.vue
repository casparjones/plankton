<script setup lang="ts">
// Task-Detail: Nur-Lesen-Ansicht eines Tasks mit allen Informationen.
import { ref, computed, watch, nextTick } from 'vue'
import { marked } from 'marked'
import type { Task, AttachmentRef } from '../types'

import { t } from '../i18n'
import { state } from '../state'
import { columnName, formatDate, labelColor } from '../utils'
import { saveTask } from '../services/project-service'
import api from '../api'

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

/** Subtasks als eigene aufklappbare Sektion (aus subtask_ids ODER parent_id). */
const subtasks = computed(() => {
  if (!task.value) return []
  // Subtasks aus subtask_ids
  const fromIds = (task.value.subtask_ids || []).map(findTask).filter(Boolean) as RelatedTask[]
  // Subtasks die via parent_id auf dieses Epic zeigen (Fallback)
  const fromParent = (state.project?.tasks || [])
    .filter((t: Task) => t.parent_id === task.value!.id && !fromIds.some(s => s.id === t.id))
    .map((t: Task) => findTask(t.id))
    .filter(Boolean) as RelatedTask[]
  return [...fromIds, ...fromParent]
})
const subtasksDone = computed(() => subtasks.value.filter(s => s.done).length)
const hasSubtasks = computed(() => subtasks.value.length > 0)
const subtasksOpen = ref(false)

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

/** Öffnet das MoveToBoardOverlay für den aktuellen Task. */
function openMoveToBoard(): void {
  if (!task.value) return
  const fn = (window as any).__openMoveToBoardOverlay
  if (typeof fn === 'function') {
    fn(task.value.id)
  }
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
  if (e.target === e.currentTarget) close()
}

// Globale Funktionen für Legacy-Kompatibilität
// @ts-ignore
window.__openTaskDetail = open
// @ts-ignore
window.__closeTaskDetail = close

defineExpose({ open, close })

// ── Attachments ──────────────────────────────────────────────────────────────

const attachments = computed<AttachmentRef[]>(() => task.value?.attachments ?? [])
const uploadError = ref('')
const uploading = ref(false)

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

async function uploadAttachment(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file || !task.value || !state.project) return
  uploading.value = true
  uploadError.value = ''
  try {
    const path = `/api/projects/${state.project._id}/tasks/${task.value.id}/attachments`
    const att = await api.upload<AttachmentRef>(path, file)
    if (!task.value.attachments) task.value.attachments = []
    task.value.attachments.push(att)
  } catch (e: unknown) {
    uploadError.value = e instanceof Error ? e.message : String(e)
  } finally {
    uploading.value = false
    input.value = ''
  }
}

async function deleteAttachment(att: AttachmentRef) {
  if (!task.value || !state.project) return
  const path = `/api/projects/${state.project._id}/tasks/${task.value.id}/attachments/${att.id}`
  await api.del(path)
  if (task.value.attachments) {
    task.value.attachments = task.value.attachments.filter(a => a.id !== att.id)
  }
}

// ── Attachment Viewer ─────────────────────────────────────────────────────────

const viewerAtt = ref<AttachmentRef | null>(null)
const viewerText = ref<string | null>(null)
const viewerMarkdown = ref<string | null>(null)
const viewerLoading = ref(false)
const viewerEl = ref<HTMLElement | null>(null)

watch(viewerAtt, (val) => {
  if (val) nextTick(() => viewerEl.value?.focus())
})

function isImage(att: AttachmentRef): boolean {
  return att.mime_type.startsWith('image/')
}

function isPdf(att: AttachmentRef): boolean {
  return att.mime_type === 'application/pdf'
}

function isViewableText(att: AttachmentRef): boolean {
  const m = att.mime_type
  return m.startsWith('text/') ||
    ['application/json', 'application/xml', 'application/javascript',
     'application/typescript', 'application/x-yaml', 'application/yaml'].some(p => m.startsWith(p))
}

async function openViewer(att: AttachmentRef): Promise<void> {
  viewerAtt.value = att
  viewerText.value = null
  viewerMarkdown.value = null
  if (isImage(att) || isPdf(att)) return
  if (isViewableText(att) || att.filename.match(/\.(md|markdown|txt|rs|ts|js|vue|py|go|sh|yaml|yml|toml|json|xml|html|css|sql|dockerfile)$/i)) {
    viewerLoading.value = true
    try {
      const url = `/api/projects/${state.project?._id}/tasks/${task.value?.id}/attachments/${att.id}`
      const res = await fetch(url)
      const content = await res.text()
      if (att.filename.match(/\.(md|markdown)$/i)) {
        viewerMarkdown.value = marked.parse(content) as string
      } else {
        viewerText.value = content
      }
    } catch {
      viewerText.value = '(Fehler beim Laden)'
    } finally {
      viewerLoading.value = false
    }
  }
}

function closeViewer(): void {
  viewerAtt.value = null
  viewerText.value = null
  viewerMarkdown.value = null
}

function viewerApiUrl(att: AttachmentRef): string {
  return `/api/projects/${state.project?._id}/tasks/${task.value?.id}/attachments/${att.id}`
}
</script>

<template>
  <div v-if="isOpen" class="fixed inset-0 bg-black/70 backdrop-blur-[2px] z-[1000] flex items-center justify-center" @click="onOverlayClick">
    <div class="bg-surface border border-border rounded-lg shadow-[0_16px_48px_rgba(0,0,0,0.5)] flex flex-col gap-3.5 max-w-[1440px] max-h-[90vh] overflow-y-auto p-6 w-[90%]">
      <div class="flex items-center justify-between">
        <span class="font-mono text-[13px] font-semibold tracking-wide uppercase text-text-dim">{{ t('taskType.' + (task?.task_type || 'task')) }}</span>
        <button class="bg-transparent border-none text-text-dim cursor-pointer text-base px-1.5 py-0.5 hover:text-text transition-colors" @click="close">&#10005;</button>
      </div>
      <div class="text-[22px] font-bold text-text leading-tight break-words">{{ task?.title }}</div>
      <div v-if="columnInfo">
        <span class="font-mono text-[10px] px-1.5 py-px rounded-sm border"
          :style="{ background: columnInfo.color + '22', borderColor: columnInfo.color, color: columnInfo.color }">{{ columnInfo.title }}</span>
      </div>
      <div class="grid grid-cols-[1fr_280px] gap-7 min-h-[300px] max-md:grid-cols-1">
        <div class="flex flex-col gap-5 min-w-0">
          <div class="flex flex-col gap-2">
            <span class="font-mono text-[11px] font-semibold uppercase tracking-wider text-text-dim border-b border-border pb-1">{{ t('taskModal.description') }}</span>
            <div class="text-sm text-text leading-relaxed break-words bg-surface-2 border border-border rounded-md px-4 py-3.5 min-h-[80px] markdown-body" v-html="renderMarkdown(task?.description)"></div>
          </div>
          <div class="flex flex-col gap-2">
            <span class="font-mono text-[11px] font-semibold uppercase tracking-wider text-text-dim border-b border-border pb-1">{{ t('taskModal.labels') }}</span>
            <div class="flex flex-wrap gap-1.5">
              <template v-if="(task?.labels || []).length">
                <span v-for="label in task!.labels" :key="label" class="text-xs px-2.5 py-[3px] rounded-xl border font-mono"
                  :style="{ background: labelColor(label).bg, borderColor: labelColor(label).border, color: labelColor(label).color }">{{ label }}</span>
              </template>
              <span v-else class="text-text-dim">–</span>
            </div>
          </div>
          <!-- Subtasks -->
          <div v-if="hasSubtasks" class="flex flex-col gap-2">
            <div class="flex items-center gap-1.5 cursor-pointer py-1.5" @click="subtasksOpen = !subtasksOpen">
              <span class="text-xs text-text-dim w-3.5">{{ subtasksOpen ? '▾' : '▸' }}</span>
              <span class="font-mono text-[11px] font-semibold uppercase tracking-wider text-text-dim cursor-pointer">{{ t('taskDetail.subtasks') }}</span>
              <span class="font-mono text-[11px] text-text-dim ml-auto">
                <span class="text-text font-semibold">{{ subtasksDone }}</span> / {{ subtasks.length }} {{ t('taskDetail.done') }}
                <span v-if="subtasksDone === subtasks.length" class="text-[#4caf50] font-bold ml-1">&#10003;</span>
              </span>
              <div class="w-[60px] h-1 bg-surface-2 rounded-sm overflow-hidden ml-2">
                <div class="h-full bg-accent rounded-sm transition-all duration-300" :style="{ width: (subtasks.length ? (subtasksDone / subtasks.length * 100) : 0) + '%' }"></div>
              </div>
            </div>
            <div v-if="subtasksOpen" class="flex flex-col gap-0.5 mt-1">
              <div v-for="sub in subtasks" :key="sub.id" class="flex items-center gap-2 px-3 py-2 bg-surface-2 border border-border rounded-md cursor-pointer transition-colors hover:border-accent" @click="openRelated(sub.id)">
                <span :class="['text-[13px] w-[18px] flex-shrink-0 text-center text-text-dim', { '!text-[#4caf50] font-bold': sub.done }]">{{ sub.done ? '✓' : '○' }}</span>
                <span class="flex-1 text-[13px] text-text overflow-hidden text-ellipsis whitespace-nowrap">{{ sub.title }}</span>
                <span class="font-mono text-[10px] text-text-dim flex-shrink-0">{{ sub.colName }}</span>
              </div>
            </div>
          </div>

          <div v-if="hasRelations" class="flex flex-col gap-2">
            <span class="font-mono text-[11px] font-semibold uppercase tracking-wider text-text-dim border-b border-border pb-1">{{ t('taskDetail.relatedTickets') }}</span>
            <div class="flex flex-col gap-3">
              <div v-for="group in relatedTickets" :key="group.label">
                <div class="font-mono text-[11px] text-text-dim uppercase tracking-wide mb-1">{{ group.icon }} {{ group.label }}</div>
                <div v-for="item in group.items" :key="item.id" class="flex items-center gap-2 px-3 py-2 bg-surface-2 border border-border rounded-md cursor-pointer transition-colors hover:border-accent" @click="openRelated(item.id)">
                  <span :class="['text-sm w-[18px] flex-shrink-0 text-center text-text-dim', { '!text-[#43a047]': item.done }]">{{ item.done ? '✓' : '○' }}</span>
                  <span v-if="item.taskType !== 'task'" class="inline-flex items-center justify-center font-mono text-[9px] font-bold w-[18px] h-[18px] rounded-sm flex-shrink-0 bg-[#1a2e1a] text-[#a5d6a7] border border-[#43a047]">{{ item.taskType === 'epic' ? 'E' : 'J' }}</span>
                  <span class="flex-1 text-[13px] text-text overflow-hidden text-ellipsis whitespace-nowrap">{{ item.title }}</span>
                  <span class="font-mono text-[10px] text-text-dim flex-shrink-0">{{ item.colName }}</span>
                </div>
              </div>
            </div>
          </div>
          <div class="flex flex-col gap-2">
            <span class="font-mono text-[11px] font-semibold uppercase tracking-wider text-text-dim border-b border-border pb-1">{{ t('taskModal.comments') }}</span>
            <div class="max-h-[300px] overflow-y-auto flex flex-col gap-1.5">
              <template v-if="comments.length">
                <div v-for="(c, i) in comments" :key="i" class="flex gap-1.5 items-baseline text-[13px] p-2 px-3 bg-surface-2 rounded-md border border-border leading-snug">
                  <template v-if="typeof c === 'object' && c !== null">
                    <span class="font-mono text-[10px] text-text-dim whitespace-nowrap flex-shrink-0">{{ c.ts }}</span>
                    <span class="text-[10px] text-text-dim whitespace-nowrap flex-shrink-0">{{ c.user }}</span>
                    <span class="text-xs text-text flex-1 markdown-body" v-html="renderMarkdown(c.msg)"></span>
                  </template>
                  <template v-else>
                    <span class="text-xs text-text flex-1 markdown-body" v-html="renderMarkdown(String(c))"></span>
                  </template>
                </div>
              </template>
              <div v-else class="text-xs text-text-dim italic">{{ t('taskModal.noComments') }}</div>
            </div>
            <div class="flex gap-2 items-end">
              <textarea
                v-model="newComment"
                :placeholder="t('taskModal.commentPlaceholder')"
                rows="2"
                class="flex-1 bg-surface-2 border border-border rounded-md text-text font-sans text-[13px] px-2.5 py-2 outline-none resize-y min-h-[36px] transition-colors focus:border-accent placeholder:text-text-dim"
                @keydown.ctrl.enter="addComment"
                @keydown.meta.enter="addComment"
              ></textarea>
              <button class="bg-accent border-none text-white font-semibold rounded-md px-3.5 py-1.5 text-xs cursor-pointer hover:opacity-85 transition-opacity disabled:opacity-50 disabled:cursor-not-allowed" @click="addComment" :disabled="!newComment.trim()">{{ t('send') }}</button>
            </div>
          </div>
        </div>
        <div class="flex flex-col gap-4">
          <div class="flex gap-2 justify-end">
            <button class="bg-surface-2 border border-border text-text-dim font-mono rounded-md px-3.5 py-1.5 text-xs cursor-pointer transition-all hover:border-accent hover:text-accent" @click="copyMcpLink" :title="mcpLinkCopied ? t('copied') : t('taskDetail.mcpLinkTitle')">
              {{ mcpLinkCopied ? '✓ ' + t('taskDetail.mcpLinkCopied') : t('taskDetail.mcpLink') }}
            </button>
            <button
              v-if="task"
              class="bg-surface-2 border border-border text-text-dim font-mono rounded-md px-3.5 py-1.5 text-xs cursor-pointer transition-all hover:border-accent hover:text-accent"
              :title="t('moveToBoard.title')"
              @click="openMoveToBoard"
            >&#8644; {{ t('moveToBoard.title') }}</button>
            <button class="bg-accent border-none text-white font-semibold rounded-md px-5 py-2 text-[13px] cursor-pointer hover:opacity-85 transition-opacity" @click="editTask">{{ t('edit') }}</button>
          </div>
          <div class="flex flex-col gap-2">
            <span class="font-mono text-[11px] font-semibold uppercase tracking-wider text-text-dim border-b border-border pb-1">{{ t('taskDetail.details') }}</span>
            <div class="grid grid-cols-2 gap-3">
              <div class="flex flex-col gap-[3px] bg-surface-2 border border-border rounded-md px-3 py-2.5">
                <span class="font-mono text-[10px] text-text-dim uppercase tracking-wide">{{ t('taskModal.type') }}</span>
                <span class="text-sm text-text font-medium">{{ task?.task_type || 'task' }}</span>
              </div>
              <div class="flex flex-col gap-[3px] bg-surface-2 border border-border rounded-md px-3 py-2.5">
                <span class="font-mono text-[10px] text-text-dim uppercase tracking-wide">{{ t('taskModal.points') }}</span>
                <span class="text-sm text-text font-medium">{{ task?.points || '–' }}</span>
              </div>
              <div class="flex flex-col gap-[3px] bg-surface-2 border border-border rounded-md px-3 py-2.5">
                <span class="font-mono text-[10px] text-text-dim uppercase tracking-wide">{{ t('taskModal.worker') }}</span>
                <span class="text-sm text-text font-medium">{{ task?.worker || '–' }}</span>
              </div>
              <div class="flex flex-col gap-[3px] bg-surface-2 border border-border rounded-md px-3 py-2.5">
                <span class="font-mono text-[10px] text-text-dim uppercase tracking-wide">{{ t('taskModal.created') }}</span>
                <span class="text-sm text-text font-medium">{{ formatDate(task?.created_at) }}</span>
              </div>
              <div class="flex flex-col gap-[3px] bg-surface-2 border border-border rounded-md px-3 py-2.5">
                <span class="font-mono text-[10px] text-text-dim uppercase tracking-wide">{{ t('taskModal.modified') }}</span>
                <span class="text-sm text-text font-medium">{{ formatDate(task?.updated_at) }}</span>
              </div>
            </div>
          </div>
          <!-- Attachments -->
          <div class="flex flex-col gap-2">
            <div class="flex items-center justify-between border-b border-border pb-1">
              <span class="font-mono text-[11px] font-semibold uppercase tracking-wider text-text-dim">Attachments</span>
              <label class="cursor-pointer text-[11px] text-accent hover:text-accent/80 font-mono">
                {{ uploading ? '…' : '+ Upload' }}
                <input type="file" class="hidden" :disabled="uploading" @change="uploadAttachment" />
              </label>
            </div>
            <div v-if="uploadError" class="text-[11px] text-red-400 font-mono">{{ uploadError }}</div>
            <div v-if="attachments.length" class="flex flex-col gap-1">
              <div v-for="att in attachments" :key="att.id"
                class="flex items-center gap-2 text-[12px] p-1.5 px-2 bg-surface-2 rounded-md border border-border hover:border-accent/50 transition-colors group">
                <button @click="openViewer(att)"
                  class="flex-1 truncate font-mono text-text text-left hover:text-accent transition-colors cursor-pointer"
                  :title="att.filename">{{ att.filename }}</button>
                <span class="text-text-dim whitespace-nowrap text-[11px]">{{ formatBytes(att.size_bytes) }}</span>
                <a :href="`/api/projects/${state.project?._id}/tasks/${task?.id}/attachments/${att.id}`"
                  download class="text-accent hover:text-accent/80 text-[11px] font-mono opacity-0 group-hover:opacity-100 transition-opacity">↓</a>
                <button @click="deleteAttachment(att)"
                  class="text-text-dim hover:text-red-400 text-[11px] font-mono ml-0.5 opacity-0 group-hover:opacity-100 transition-opacity">✕</button>
              </div>
            </div>
            <div v-else-if="!uploading" class="text-xs text-text-dim italic">Keine Anhänge</div>
          </div>

          <div class="flex flex-col gap-2">
            <span class="font-mono text-[11px] font-semibold uppercase tracking-wider text-text-dim border-b border-border pb-1">{{ t('taskModal.logs') }}</span>
            <div class="max-h-[250px] overflow-y-auto flex flex-col gap-1.5">
              <template v-if="logs.length">
                <div v-for="(l, i) in logs" :key="i" class="text-[11px] text-text-dim p-1 px-2 bg-surface-2 rounded-sm border border-border font-mono flex gap-1.5 items-baseline">
                  <template v-if="typeof l === 'object'">
                    <span class="text-[10px] text-text-dim whitespace-nowrap flex-shrink-0">{{ l.ts }}</span>
                    <span class="text-xs text-text flex-1 overflow-hidden text-ellipsis">{{ l.msg }}</span>
                    <span class="text-[10px] text-text-dim whitespace-nowrap flex-shrink-0">{{ l.user }}</span>
                  </template>
                  <template v-else>{{ l }}</template>
                </div>
              </template>
              <div v-else class="text-xs text-text-dim italic">{{ t('taskModal.noLogs') }}</div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>

  <!-- Attachment Viewer Overlay -->
  <Teleport to="body">
    <div
      v-if="viewerAtt"
      ref="viewerEl"
      tabindex="-1"
      class="fixed inset-0 bg-black/92 z-[2000] flex flex-col outline-none"
      @click.self="closeViewer"
      @keydown.esc="closeViewer"
    >
      <!-- Header -->
      <div class="flex items-center justify-between px-5 py-3 border-b border-white/10 flex-shrink-0 bg-black/40">
        <span class="font-mono text-[13px] text-white/60 truncate max-w-[60%]">{{ viewerAtt.filename }}</span>
        <div class="flex items-center gap-3 flex-shrink-0">
          <span class="text-[11px] text-white/30 font-mono">{{ formatBytes(viewerAtt.size_bytes) }}</span>
          <a :href="viewerApiUrl(viewerAtt)" download
            class="font-mono text-[11px] text-accent hover:text-accent/80 border border-accent/30 rounded px-2.5 py-1 transition-colors">↓ Download</a>
          <button @click="closeViewer" class="text-white/40 hover:text-white text-lg leading-none px-1.5 py-0.5 transition-colors">✕</button>
        </div>
      </div>
      <!-- Content -->
      <div class="flex-1 overflow-auto flex items-center justify-center p-6 min-h-0" @click.self="closeViewer">
        <!-- Image -->
        <img
          v-if="isImage(viewerAtt)"
          :src="viewerApiUrl(viewerAtt)"
          class="max-w-full max-h-full object-contain rounded shadow-2xl select-none"
          :alt="viewerAtt.filename"
        />
        <!-- PDF -->
        <iframe
          v-else-if="isPdf(viewerAtt)"
          :src="viewerApiUrl(viewerAtt)"
          class="w-full h-full border-none rounded"
          style="max-width: 1000px;"
        />
        <!-- Loading -->
        <div v-else-if="viewerLoading" class="text-white/40 font-mono text-sm">Laden…</div>
        <!-- Markdown -->
        <div
          v-else-if="viewerMarkdown !== null"
          class="max-w-4xl w-full bg-[#161b22] border border-white/10 rounded-lg px-10 py-8 overflow-auto max-h-full markdown-body"
          v-html="viewerMarkdown"
        />
        <!-- Code / Text -->
        <pre
          v-else-if="viewerText !== null"
          class="w-full max-w-full bg-[#0d1117] border border-white/10 rounded-lg p-5 font-mono text-[13px] text-[#e6edf3] overflow-auto max-h-full leading-relaxed"
        >{{ viewerText }}</pre>
        <!-- Unsupported -->
        <div v-else class="flex flex-col items-center gap-3 text-white/30">
          <span class="text-4xl">📎</span>
          <span class="font-mono text-sm">Keine Vorschau verfügbar</span>
          <a :href="viewerApiUrl(viewerAtt)" download
            class="text-accent hover:text-accent/80 font-mono text-[13px] border border-accent/30 rounded px-4 py-2 transition-colors">↓ Herunterladen</a>
        </div>
      </div>
    </div>
  </Teleport>
</template>
