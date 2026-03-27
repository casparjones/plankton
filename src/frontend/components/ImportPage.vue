<script setup lang="ts">
// Mobile-optimierte Import-Seite für Task-Erstellung via Smartphone.
import { ref, computed, onMounted } from 'vue'
import { t } from '../i18n'
import api from '../api'
import type { ProjectDoc } from '../types'

const projects = ref<ProjectDoc[]>([])
const selectedProjectId = ref('')
const newProjectName = ref('')
const showNewProject = ref(false)
const jsonInput = ref('')
const validationError = ref('')
const importResult = ref('')
const promptCopied = ref(false)

const selectedProject = computed(() =>
  projects.value.find(p => p._id === selectedProjectId.value)
)

async function loadProjects(): Promise<void> {
  projects.value = await api.get<ProjectDoc[]>('/api/projects')
  if (projects.value.length && !selectedProjectId.value) {
    selectedProjectId.value = projects.value[0]._id
  }
}

async function createProject(): Promise<void> {
  const name = newProjectName.value.trim()
  if (!name) return
  const created = await api.post<ProjectDoc>('/api/projects', {
    _id: '', title: name, columns: [
      { id: crypto.randomUUID(), title: 'Todo', order: 0, color: '#90CAF9', hidden: false, slug: '', locked: false },
      { id: crypto.randomUUID(), title: 'In Progress', order: 1, color: '#FFCC80', hidden: false, slug: '', locked: false },
      { id: crypto.randomUUID(), title: 'Testing', order: 2, color: '#CE93D8', hidden: false, slug: '', locked: false },
      { id: crypto.randomUUID(), title: 'Done', order: 3, color: '#A5D6A7', hidden: false, slug: '', locked: false },
      { id: crypto.randomUUID(), title: '_archive', order: 99, color: '#444', hidden: true, slug: '', locked: false },
    ], users: [], tasks: []
  })
  await loadProjects()
  selectedProjectId.value = created._id
  newProjectName.value = ''
  showNewProject.value = false
}

function validate(): boolean {
  validationError.value = ''
  const raw = jsonInput.value.trim()
  if (!raw) { validationError.value = 'JSON must not be empty'; return false }
  try {
    const parsed = JSON.parse(raw)
    const tasks = Array.isArray(parsed) ? parsed : [parsed]
    for (const task of tasks) {
      if (!task.title || typeof task.title !== 'string') {
        validationError.value = `Task missing "title": ${JSON.stringify(task).substring(0, 60)}…`
        return false
      }
    }
    return true
  } catch (e) {
    validationError.value = `Invalid JSON: ${(e as Error).message}`
    return false
  }
}

async function doImport(): Promise<void> {
  if (!selectedProjectId.value) { validationError.value = 'Please select a project'; return }
  if (!validate()) return
  const parsed = JSON.parse(jsonInput.value.trim())
  const tasks = Array.isArray(parsed) ? parsed : [parsed]
  try {
    const resp = await api.post<{ imported: number; warnings: string[]; errors: string[] }>(
      `/api/projects/${selectedProjectId.value}/import`,
      { tasks }
    )
    importResult.value = `${resp.imported} task(s) imported` +
      (resp.warnings.length ? `\n⚠ ${resp.warnings.join('\n⚠ ')}` : '') +
      (resp.errors.length ? `\n✗ ${resp.errors.join('\n✗ ')}` : '')
    jsonInput.value = ''
    validationError.value = ''
  } catch (e) {
    validationError.value = `Import failed: ${(e as Error).message}`
  }
}

async function pasteFromClipboard(): Promise<void> {
  try {
    jsonInput.value = await navigator.clipboard.readText()
  } catch {
    validationError.value = 'Clipboard access not allowed'
  }
}

const supervisorPrompt = computed(() => {
  if (!selectedProject.value || !jsonInput.value.trim()) return ''
  return `You are the Plankton Supervisor Agent for project "${selectedProject.value.title}".
The following tasks have been submitted via mobile import:

${jsonInput.value.trim()}

Process each task: validate structure, assign to the appropriate agent (Architect / Developer / Tester), and update the board accordingly.`
})

function copyPrompt(): void {
  if (!supervisorPrompt.value) return
  const text = supervisorPrompt.value
  if (navigator.clipboard?.writeText) {
    navigator.clipboard.writeText(text).then(done).catch(fallback)
  } else {
    fallback()
  }
  function done() { promptCopied.value = true; setTimeout(() => { promptCopied.value = false }, 2000) }
  function fallback() {
    const ta = document.createElement('textarea')
    ta.value = text; ta.style.position = 'fixed'; ta.style.opacity = '0'
    document.body.appendChild(ta); ta.select(); document.execCommand('copy')
    document.body.removeChild(ta); done()
  }
}

onMounted(loadProjects)
</script>

<template>
  <div class="min-h-screen bg-bg pb-10 max-w-[600px] mx-auto">
    <header class="flex items-center gap-4 px-5 py-4 border-b border-border bg-surface sticky top-0 z-10">
      <a href="/" class="font-mono text-sm text-accent no-underline">{{ t('import.backToBoard') }}</a>
      <span class="font-mono text-base font-semibold text-text">{{ t('import.mobileImport') }}</span>
    </header>

    <!-- Projekt auswählen -->
    <section class="px-5 py-5 flex flex-col gap-2.5">
      <label class="font-mono text-[11px] text-text-dim uppercase tracking-wider font-semibold">{{ t('import.projectLabel') }}</label>
      <select v-model="selectedProjectId"
        class="bg-surface-2 border border-border rounded-md text-text text-base px-3.5 py-3 outline-none w-full font-sans focus:border-accent">
        <option v-for="p in projects" :key="p._id" :value="p._id">{{ p.title }}</option>
      </select>
      <button v-if="!showNewProject" class="bg-transparent border-none text-accent text-[13px] font-mono cursor-pointer p-0 py-1 text-left hover:underline" @click="showNewProject = true">{{ t('import.newProject') }}</button>
      <div v-if="showNewProject" class="flex gap-2 items-center flex-wrap">
        <input v-model="newProjectName" :placeholder="t('sidebar.projectPlaceholder')" @keydown.enter="createProject"
          class="bg-surface-2 border border-border rounded-md text-text text-base px-3.5 py-3 outline-none w-full font-sans focus:border-accent" />
        <button class="px-3.5 py-2 text-[13px] font-mono bg-surface-2 border border-border rounded-md text-text-dim cursor-pointer min-h-[44px] hover:border-accent hover:text-accent" @click="createProject">{{ t('create') }}</button>
        <button class="px-3.5 py-2 text-[13px] font-mono bg-transparent border-transparent rounded-md text-text-dim cursor-pointer min-h-[44px]" @click="showNewProject = false">✕</button>
      </div>
    </section>

    <!-- JSON Eingabe -->
    <section class="px-5 py-5 flex flex-col gap-2.5">
      <label class="font-mono text-[11px] text-text-dim uppercase tracking-wider font-semibold">{{ t('import.tasksJson') }}</label>
      <textarea
        v-model="jsonInput"
        :class="['bg-surface-2 border rounded-md text-text text-base px-3.5 py-3 outline-none w-full font-mono resize-y min-h-[120px] leading-relaxed focus:border-accent placeholder:text-text-dim',
          validationError ? 'border-[#e53935]' : 'border-border']"
        rows="8"
        :placeholder='`[{"title": "My Task", "labels": ["feature"], "points": 5}]`'
        spellcheck="false"
      ></textarea>
      <div class="flex gap-2 items-center flex-wrap">
        <button class="px-3.5 py-2 text-[13px] font-mono bg-surface-2 border border-border rounded-md text-text-dim cursor-pointer min-h-[44px] hover:border-accent hover:text-accent" @click="pasteFromClipboard">{{ t('import.paste') }}</button>
        <button class="px-3.5 py-2 text-[13px] font-mono bg-surface-2 border border-border rounded-md text-text-dim cursor-pointer min-h-[44px] hover:border-accent hover:text-accent" @click="validate">{{ t('import.check') }}</button>
        <button class="bg-accent text-bg border-accent border rounded-md font-semibold px-4.5 py-2.5 text-sm cursor-pointer min-h-[44px] ml-auto hover:opacity-90 transition-opacity" @click="doImport">{{ t('import.importBtn') }}</button>
      </div>
      <div v-if="validationError" class="text-[#ff8a80] px-3 py-2 bg-[#3a1c1c] border border-[#e53935] rounded-md whitespace-pre-wrap text-[13px]">{{ validationError }}</div>
      <div v-if="importResult" class="text-[#a5d6a7] text-[13px] px-3 py-2 bg-[#1a2e1a] border border-[#43a047] rounded-md whitespace-pre-wrap">{{ importResult }}</div>
    </section>

    <!-- Supervisor Prompt -->
    <section v-if="supervisorPrompt" class="px-5 py-5 flex flex-col gap-2.5">
      <label class="font-mono text-[11px] text-text-dim uppercase tracking-wider font-semibold">{{ t('import.supervisorPrompt') }}</label>
      <textarea class="bg-surface border border-border rounded-md text-text text-[13px] px-3.5 py-3 outline-none w-full font-mono resize-y min-h-[120px] leading-relaxed opacity-80" :value="supervisorPrompt" rows="6" readonly></textarea>
      <button class="bg-transparent border border-border rounded-md text-text-dim cursor-pointer font-sans text-xs px-2.5 py-1 transition-all hover:border-accent hover:text-accent" @click="copyPrompt">{{ promptCopied ? '✓ ' + t('copied') : t('import.copyPrompt') }}</button>
    </section>
  </div>
</template>
