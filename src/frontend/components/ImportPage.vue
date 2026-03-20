<script setup lang="ts">
// Mobile-optimierte Import-Seite für Task-Erstellung via Smartphone.
import { ref, computed, onMounted } from 'vue'
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
  if (!raw) { validationError.value = 'JSON darf nicht leer sein'; return false }
  try {
    const parsed = JSON.parse(raw)
    const tasks = Array.isArray(parsed) ? parsed : [parsed]
    for (const t of tasks) {
      if (!t.title || typeof t.title !== 'string') {
        validationError.value = `Task fehlt "title": ${JSON.stringify(t).substring(0, 60)}…`
        return false
      }
    }
    return true
  } catch (e) {
    validationError.value = `Ungültiges JSON: ${(e as Error).message}`
    return false
  }
}

async function doImport(): Promise<void> {
  if (!selectedProjectId.value) { validationError.value = 'Bitte Projekt auswählen'; return }
  if (!validate()) return
  const parsed = JSON.parse(jsonInput.value.trim())
  const tasks = Array.isArray(parsed) ? parsed : [parsed]
  try {
    const resp = await api.post<{ imported: number; warnings: string[]; errors: string[] }>(
      `/api/projects/${selectedProjectId.value}/import`,
      { tasks }
    )
    importResult.value = `${resp.imported} Task(s) importiert` +
      (resp.warnings.length ? `\n⚠ ${resp.warnings.join('\n⚠ ')}` : '') +
      (resp.errors.length ? `\n✗ ${resp.errors.join('\n✗ ')}` : '')
    jsonInput.value = ''
    validationError.value = ''
  } catch (e) {
    validationError.value = `Import fehlgeschlagen: ${(e as Error).message}`
  }
}

async function pasteFromClipboard(): Promise<void> {
  try {
    jsonInput.value = await navigator.clipboard.readText()
  } catch {
    validationError.value = 'Clipboard-Zugriff nicht erlaubt'
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
  <div class="import-page">
    <header class="import-header">
      <a href="/" class="import-back">← Board</a>
      <span class="import-title">Mobile Import</span>
    </header>

    <!-- Projekt auswählen -->
    <section class="import-section">
      <label class="import-label">Projekt</label>
      <select v-model="selectedProjectId" class="import-select">
        <option v-for="p in projects" :key="p._id" :value="p._id">{{ p.title }}</option>
      </select>
      <button v-if="!showNewProject" class="import-link" @click="showNewProject = true">+ Neues Projekt</button>
      <div v-if="showNewProject" class="import-row">
        <input v-model="newProjectName" class="import-input" placeholder="Projektname…" @keydown.enter="createProject" />
        <button class="import-btn-sm" @click="createProject">Erstellen</button>
        <button class="import-btn-sm import-btn-ghost" @click="showNewProject = false">✕</button>
      </div>
    </section>

    <!-- JSON Eingabe -->
    <section class="import-section">
      <label class="import-label">Tasks (JSON)</label>
      <textarea
        v-model="jsonInput"
        class="import-textarea"
        :class="{ 'import-error-border': validationError }"
        rows="8"
        placeholder='[{"title": "Mein Task", "labels": ["feature"], "points": 5}]'
        spellcheck="false"
      ></textarea>
      <div class="import-row">
        <button class="import-btn-sm" @click="pasteFromClipboard">Einfügen</button>
        <button class="import-btn-sm" @click="validate">Prüfen</button>
        <button class="import-btn import-btn-primary" @click="doImport">Importieren</button>
      </div>
      <div v-if="validationError" class="import-error">{{ validationError }}</div>
      <div v-if="importResult" class="import-success">{{ importResult }}</div>
    </section>

    <!-- Supervisor Prompt -->
    <section v-if="supervisorPrompt" class="import-section">
      <label class="import-label">Supervisor Prompt</label>
      <textarea class="import-textarea import-textarea-ro" :value="supervisorPrompt" rows="6" readonly></textarea>
      <button class="import-btn" @click="copyPrompt">{{ promptCopied ? '✓ Kopiert' : 'Prompt kopieren' }}</button>
    </section>
  </div>
</template>
