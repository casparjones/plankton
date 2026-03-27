<script setup lang="ts">
// Task-Modal: Erstellen und Bearbeiten von Tasks.
import { ref, computed, watch } from 'vue'
import type { Task } from '../types'

import { state } from '../state'
import { columnName, formatDate } from '../utils'
// @ts-ignore
import { saveTask, createTaskViaApi, deleteTask } from '../services/project-service'
import { toastConfirm } from '../toast'
import { t } from '../i18n'

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
    order: 0,
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
  const ok = await toastConfirm(t('taskModal.deleteConfirm', { title: editingTask.value.title }))
  if (ok) {
    deleteTask(editingTask.value.id)
    close()
  }
}

function addComment(): void {
  const text = newComment.value.trim()
  if (!text || !editingTask.value) return
  const userName = state.currentUser?.display_name || state.currentUser?.username || 'anonymous'
  const now = new Date()
  const ts = `${String(now.getMonth()+1).padStart(2,'0')}-${String(now.getDate()).padStart(2,'0')} ${String(now.getHours()).padStart(2,'0')}:${String(now.getMinutes()).padStart(2,'0')}`
  comments.value.push({ ts, user: userName, msg: text } as any)
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
  <div v-if="isOpen" class="fixed inset-0 bg-black/70 backdrop-blur-[2px] z-[1000] flex items-center justify-center" @click="onOverlayClick">
    <div class="bg-surface border border-border rounded-lg shadow-[0_16px_48px_rgba(0,0,0,0.5)] flex flex-col gap-3.5 max-w-[1000px] p-6 w-[90%]">
      <div class="flex items-center justify-between">
        <span class="font-mono text-[13px] font-semibold tracking-wide uppercase text-text-dim">{{ isNew ? t('taskModal.newTask') : t('taskModal.editTask') }}</span>
        <button class="bg-transparent border-none text-text-dim cursor-pointer text-base px-1.5 py-0.5 hover:text-text transition-colors" @click="close">&#10005;</button>
      </div>
      <div class="grid grid-cols-[1fr_260px] gap-6 max-md:grid-cols-1">
        <div class="flex flex-col gap-3">
          <label class="flex flex-col gap-1.5 text-xs text-text-dim font-mono uppercase tracking-wide">{{ t('taskModal.title') }}
            <input v-model="title" type="text" class="task-modal-title-input bg-surface-2 border border-border rounded-md text-text font-sans text-sm px-3 py-2 outline-none transition-colors focus:border-accent" />
          </label>
          <label class="flex flex-col gap-1.5 text-xs text-text-dim font-mono uppercase tracking-wide">{{ t('taskModal.description') }}
            <textarea v-model="description" rows="14" class="bg-surface-2 border border-border rounded-md text-text font-sans text-sm px-3 py-2 outline-none resize-y transition-colors focus:border-accent"></textarea>
          </label>
          <label class="flex flex-col gap-1.5 text-xs text-text-dim font-mono uppercase tracking-wide">{{ t('taskModal.labels') }} <small class="normal-case font-sans">({{ t('taskModal.labelsHint') }})</small>
            <input v-model="labels" type="text" class="bg-surface-2 border border-border rounded-md text-text font-sans text-sm px-3 py-2 outline-none transition-colors focus:border-accent" />
          </label>
          <div class="flex flex-col gap-1.5">
            <span class="font-mono text-[10px] text-text-dim uppercase tracking-wide">{{ t('taskModal.comments') }}</span>
            <div class="max-h-[200px] overflow-y-auto flex flex-col gap-1">
              <template v-if="comments.length">
                <div v-for="(c, i) in comments" :key="i" class="flex gap-1.5 items-baseline text-xs p-1.5 px-2 bg-surface-2 rounded-sm border border-border">
                  <template v-if="typeof c === 'object' && c !== null">
                    <span class="font-mono text-[10px] text-text-dim whitespace-nowrap flex-shrink-0">{{ c.ts }}</span>
                    <span class="text-[10px] text-text-dim whitespace-nowrap flex-shrink-0">{{ c.user }}</span>
                    <span class="text-xs text-text flex-1 overflow-hidden text-ellipsis">{{ c.msg }}</span>
                  </template>
                  <template v-else>{{ c }}</template>
                </div>
              </template>
              <div v-else class="text-xs text-text-dim italic">{{ t('taskModal.noComments') }}</div>
            </div>
            <div class="flex gap-1">
              <input v-model="newComment" type="text" :placeholder="t('taskModal.commentPlaceholder')" @keydown.enter="addComment"
                class="flex-1 bg-surface-2 border border-border rounded-md text-text text-xs px-2 py-1 outline-none focus:border-accent" />
              <button class="bg-accent-dim border border-accent rounded-md text-text cursor-pointer text-sm px-2 py-0.5 transition-colors hover:bg-accent" @click="addComment">+</button>
            </div>
          </div>
        </div>
        <div v-if="!isNew" class="flex flex-col gap-3">
          <label class="flex flex-col gap-1.5 text-xs text-text-dim font-mono uppercase tracking-wide">{{ t('taskModal.type') }}
            <select v-model="taskType" class="bg-surface-2 border border-border rounded-md text-text font-sans text-sm px-3 py-2 outline-none focus:border-accent">
              <option value="task">Task</option>
              <option value="epic">Epic</option>
              <option value="job">Job</option>
            </select>
          </label>
          <label class="flex flex-col gap-1.5 text-xs text-text-dim font-mono uppercase tracking-wide">{{ t('taskModal.parentEpic') }}
            <select v-model="parentId" class="bg-surface-2 border border-border rounded-md text-text font-sans text-sm px-3 py-2 outline-none focus:border-accent">
              <option value="">–</option>
              <option v-for="e in epics" :key="e.id" :value="e.id">{{ e.title }}</option>
            </select>
          </label>
          <label class="flex flex-col gap-1.5 text-xs text-text-dim font-mono uppercase tracking-wide">{{ t('taskModal.points') }} <small class="normal-case font-sans">(0–100)</small>
            <input v-model.number="points" type="number" min="0" max="100" class="bg-surface-2 border border-border rounded-md text-text font-sans text-sm px-3 py-2 outline-none transition-colors focus:border-accent" />
          </label>
          <label class="flex flex-col gap-1.5 text-xs text-text-dim font-mono uppercase tracking-wide">{{ t('taskModal.worker') }}
            <input v-model="worker" type="text" class="bg-surface-2 border border-border rounded-md text-text font-sans text-sm px-3 py-2 outline-none transition-colors focus:border-accent" />
          </label>
          <div class="flex flex-col gap-0.5">
            <span class="font-mono text-[10px] text-text-dim uppercase tracking-wide">{{ t('taskModal.created') }}</span>
            <span class="text-xs text-text">{{ createdAt }}</span>
          </div>
          <div class="flex flex-col gap-0.5">
            <span class="font-mono text-[10px] text-text-dim uppercase tracking-wide">{{ t('taskModal.modified') }}</span>
            <span class="text-xs text-text">{{ updatedAt }}</span>
          </div>
          <div class="flex flex-col gap-0.5">
            <span class="font-mono text-[10px] text-text-dim uppercase tracking-wide">{{ t('taskModal.previousColumn') }}</span>
            <span class="text-xs text-text">{{ previousRow }}</span>
          </div>
          <div class="flex flex-col gap-1.5" v-if="otherTasks.length">
            <span class="font-mono text-[10px] text-text-dim uppercase tracking-wide">{{ t('taskModal.blockedBy') }}</span>
            <div class="relative">
              <div class="flex flex-wrap gap-1 mb-1" v-if="selectedBlockers.length">
                <span v-for="t in selectedBlockers" :key="t.id" class="inline-flex items-center gap-1 bg-surface-2 border border-border rounded-sm px-1.5 py-0.5 text-[11px] font-sans text-text max-w-full">
                  {{ t.title }}
                  <button type="button" class="bg-transparent border-none text-text-dim cursor-pointer text-[13px] px-px leading-none hover:text-danger" @click="removeBlocker(t.id)">&times;</button>
                </span>
              </div>
              <div class="relative">
                <input
                  v-model="blockedBySearch"
                  type="text"
                  class="w-full bg-surface-2 border border-border rounded-md text-text text-xs px-2 py-[5px] outline-none focus:border-accent"
                  :placeholder="t('taskModal.searchTask')"
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
          <div class="flex flex-col gap-1.5">
            <span class="font-mono text-[10px] text-text-dim uppercase tracking-wide">{{ t('taskModal.logs') }}</span>
            <div class="max-h-[140px] overflow-y-auto flex flex-col gap-1">
              <template v-if="logs.length">
                <div v-for="(l, i) in logs" :key="i" class="flex gap-1.5 items-baseline text-xs p-1 px-2 bg-surface-2 rounded-sm border border-border">
                  <template v-if="typeof l === 'object'">
                    <span class="font-mono text-[10px] text-text-dim whitespace-nowrap flex-shrink-0">{{ l.ts }}</span>
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
        <div v-else class="flex flex-col gap-3">
          <label class="flex flex-col gap-1.5 text-xs text-text-dim font-mono uppercase tracking-wide">{{ t('taskModal.type') }}
            <select v-model="taskType" class="bg-surface-2 border border-border rounded-md text-text font-sans text-sm px-3 py-2 outline-none focus:border-accent">
              <option value="task">Task</option>
              <option value="epic">Epic</option>
              <option value="job">Job</option>
            </select>
          </label>
          <label class="flex flex-col gap-1.5 text-xs text-text-dim font-mono uppercase tracking-wide">{{ t('taskModal.parentEpic') }}
            <select v-model="parentId" class="bg-surface-2 border border-border rounded-md text-text font-sans text-sm px-3 py-2 outline-none focus:border-accent">
              <option value="">–</option>
              <option v-for="e in epics" :key="e.id" :value="e.id">{{ e.title }}</option>
            </select>
          </label>
          <label class="flex flex-col gap-1.5 text-xs text-text-dim font-mono uppercase tracking-wide">{{ t('taskModal.points') }} <small class="normal-case font-sans">(0–100)</small>
            <input v-model.number="points" type="number" min="0" max="100" class="bg-surface-2 border border-border rounded-md text-text font-sans text-sm px-3 py-2 outline-none transition-colors focus:border-accent" />
          </label>
          <label class="flex flex-col gap-1.5 text-xs text-text-dim font-mono uppercase tracking-wide">{{ t('taskModal.worker') }}
            <input v-model="worker" type="text" class="bg-surface-2 border border-border rounded-md text-text font-sans text-sm px-3 py-2 outline-none transition-colors focus:border-accent" />
          </label>
        </div>
      </div>
      <div class="flex gap-2 justify-end mt-1">
        <button class="bg-accent border-none text-white font-semibold rounded-md px-5 py-2 text-[13px] cursor-pointer hover:opacity-85 transition-opacity" @click="save">{{ t('save') }}</button>
        <button v-if="!isNew" class="bg-transparent border border-danger text-danger rounded-md px-5 py-2 text-[13px] cursor-pointer hover:bg-danger/10 transition-colors" @click="handleDelete">{{ t('delete') }}</button>
      </div>
    </div>
  </div>
</template>
