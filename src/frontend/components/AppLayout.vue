<script setup lang="ts">
// Haupt-Layout der Plankton-App: Sidebar, Header, Board und alle Modals.
// Task-Modal und Task-Detail sind Vue-Komponenten, die übrigen Modals
// nutzen weiterhin Legacy-DOM mit Event-Listenern in onMounted().

import { ref, onMounted } from 'vue'
import KanbanBoard from './KanbanBoard.vue'
import TaskModal from './TaskModal.vue'
import TaskDetail from './TaskDetail.vue'
import type { Task } from '../types'

import { t, useI18n } from '../i18n'

const { locale, setLocale, locales } = useI18n()
import { state } from '../state'
import { doLogout } from '../components/auth'
import { updateBulkBar, bulkDeleteSelected } from '../components/bulk-actions'
// @ts-ignore
import { closeColumnModal, saveColumnModal, selectColor } from '../components/column-modal'
// @ts-ignore
import { openProjectDropdown, closeProjectMenu, copyProjectJson, importProjectJson, saveProjectJson, saveProjectTitle, closePromptModal, initPromptTabs, closeCliModal, initCliModal } from '../components/project-menu'
// @ts-ignore
import { toggleJsonView } from '../components/json-view'
// @ts-ignore
import { openAdminModal, closeAdminModal, showAdminForm, saveAdminForm, switchAdminTab, createToken, handleTokenAction, handleAdminUserAction } from '../components/admin'
// @ts-ignore
import { openPasswordModal, closePasswordModal, savePassword } from '../components/password-modal'
// @ts-ignore
import { openImportModal, closeImportModal, validateImport, executeImport } from '../components/import-modal'
// @ts-ignore
import { openGitModal, closeGitModal, saveGitConfig, triggerGitSync } from '../components/git-settings'
// @ts-ignore
import { createProject } from '../services/project-service'

/** Triggert Board-Refresh via globale Bridge-Funktion. */
function triggerBoardRefresh(): void {
  if (typeof window.__kanbanRefresh === 'function') window.__kanbanRefresh()
}

const props = defineProps<{
  onLogout: () => void
}>()

// Refs für Vue-Komponenten
const taskModalRef = ref<InstanceType<typeof TaskModal> | null>(null)
const taskDetailRef = ref<InstanceType<typeof TaskDetail> | null>(null)

/** Projekt erstellen via Eingabefeld. */
function handleCreateProject(): void {
  const input = document.getElementById('new-project-input') as HTMLInputElement
  if (input && input.value.trim()) {
    createProject(input.value.trim())
    input.value = ''
  }
}

/** Task-Detail → Bearbeiten: Öffnet das Task-Modal. */
function onEditFromDetail(task: Task): void {
  taskModalRef.value?.openEdit(task)
}

/** Registriert Event-Listener für Legacy-Modals. */
onMounted(() => {
  // Projekt erstellen.
  document.getElementById('new-project-input')?.addEventListener('keydown', (e: KeyboardEvent) => {
    if (e.key === 'Enter') handleCreateProject()
  })

  // Bulk-Aktionen.
  document.getElementById('bulk-delete-btn')?.addEventListener('click', bulkDeleteSelected)
  document.getElementById('bulk-cancel-btn')?.addEventListener('click', () => {
    state.selectedTasks.clear()
    triggerBoardRefresh()
  })

  // Spalten-Modal.
  document.getElementById('col-modal-close')?.addEventListener('click', closeColumnModal)
  document.getElementById('column-modal')?.addEventListener('click', (e: Event) => {
    if ((e.target as HTMLElement).id === 'column-modal') closeColumnModal()
  })
  document.getElementById('col-modal-save')?.addEventListener('click', saveColumnModal)
  document.getElementById('col-modal-title')?.addEventListener('keydown', (e: KeyboardEvent) => {
    if (e.key === 'Enter') saveColumnModal()
    if (e.key === 'Escape') closeColumnModal()
  })
  document.getElementById('col-modal-colors')?.addEventListener('click', (e: Event) => {
    const swatch = (e.target as HTMLElement).closest('.color-swatch') as HTMLElement | null
    if (!swatch) return
    selectColor(swatch.dataset.color)
  })

  // Projekt-Menü.
  document.getElementById('project-menu-btn')?.addEventListener('click', (e: Event) => {
    e.stopPropagation()
    openProjectDropdown()
  })

  // Projekt-Editieren Modal.
  document.getElementById('proj-modal-close')?.addEventListener('click', closeProjectMenu)
  document.getElementById('project-modal')?.addEventListener('click', (e: Event) => {
    if ((e.target as HTMLElement).id === 'project-modal') closeProjectMenu()
  })
  document.getElementById('proj-modal-copy')?.addEventListener('click', copyProjectJson)
  document.getElementById('proj-modal-import')?.addEventListener('click', importProjectJson)
  document.getElementById('proj-modal-save')?.addEventListener('click', saveProjectJson)
  document.getElementById('proj-modal-title')?.addEventListener('keydown', (e: KeyboardEvent) => {
    if (e.key === 'Enter') saveProjectTitle()
  })
  document.getElementById('proj-view-toggle')?.addEventListener('click', toggleJsonView)

  // Prompt-Modal (Tabs + Events).
  initPromptTabs()

  // CLI-Modal (Install CLI).
  initCliModal()

  // User-Aktionen.
  document.getElementById('logout-btn')?.addEventListener('click', () => doLogout(props.onLogout))
  document.getElementById('password-btn')?.addEventListener('click', () => openPasswordModal(false))
  document.getElementById('admin-btn')?.addEventListener('click', openAdminModal)

  // Admin-Modal.
  document.getElementById('admin-modal-close')?.addEventListener('click', closeAdminModal)
  document.getElementById('admin-modal')?.addEventListener('click', (e: Event) => {
    if ((e.target as HTMLElement).id === 'admin-modal') closeAdminModal()
  })
  document.getElementById('admin-add-user-btn')?.addEventListener('click', () => showAdminForm(null))
  document.getElementById('admin-form-save')?.addEventListener('click', saveAdminForm)
  document.getElementById('admin-form-cancel')?.addEventListener('click', () => openAdminModal())
  document.querySelectorAll('.admin-tab').forEach((tab: Element) => {
    tab.addEventListener('click', () => switchAdminTab((tab as HTMLElement).dataset.tab))
  })
  document.getElementById('admin-create-token-btn')?.addEventListener('click', createToken)
  document.getElementById('admin-token-list')?.addEventListener('click', async (e: Event) => {
    const btn = (e.target as HTMLElement).closest('[data-token-action]') as HTMLElement | null
    if (!btn) return
    handleTokenAction(btn.dataset.tokenAction, btn.dataset.tid)
  })
  document.getElementById('admin-user-list')?.addEventListener('click', async (e: Event) => {
    const btn = (e.target as HTMLElement).closest('[data-admin-action]') as HTMLElement | null
    if (!btn) return
    handleAdminUserAction(btn.dataset.adminAction, btn.dataset.uid)
  })

  // Passwort-Modal.
  document.getElementById('pw-modal-close')?.addEventListener('click', closePasswordModal)
  document.getElementById('password-modal')?.addEventListener('click', (e: Event) => {
    if ((e.target as HTMLElement).id === 'password-modal') closePasswordModal()
  })
  document.getElementById('pw-save-btn')?.addEventListener('click', savePassword)

  // Import-Modal.
  document.getElementById('import-btn')?.addEventListener('click', openImportModal)
  document.getElementById('import-modal-close')?.addEventListener('click', closeImportModal)
  document.getElementById('import-modal')?.addEventListener('click', (e: Event) => {
    if ((e.target as HTMLElement).id === 'import-modal') closeImportModal()
  })
  document.getElementById('import-validate-btn')?.addEventListener('click', validateImport)
  document.getElementById('import-start-btn')?.addEventListener('click', executeImport)

  // Git-Status-Icon + Modal.
  document.getElementById('git-status-icon')?.addEventListener('click', () => openGitModal())
  document.getElementById('git-modal-close')?.addEventListener('click', closeGitModal)
  document.getElementById('git-modal')?.addEventListener('click', (e: Event) => {
    if ((e.target as HTMLElement).id === 'git-modal') closeGitModal()
  })
  document.getElementById('git-save-btn')?.addEventListener('click', saveGitConfig)
  document.getElementById('git-sync-btn')?.addEventListener('click', triggerGitSync)
})
</script>

<template>
  <div class="flex h-screen overflow-hidden">
    <!-- Sidebar -->
    <aside class="sidebar w-[220px] min-w-[220px] bg-surface border-r border-border flex flex-col transition-colors duration-250">
      <div class="px-4 py-[18px] pb-3.5 border-b border-border flex items-center justify-between">
        <span class="font-mono text-lg font-semibold tracking-wide text-accent flex items-center gap-2.5">
          <img src="/icons/logo.svg" alt="" class="w-8 h-8" /> Plankton
        </span>
        <button id="theme-toggle" class="bg-transparent border border-border rounded-md text-text-dim cursor-pointer text-sm px-2 py-0.5 leading-none transition-all hover:border-accent hover:text-accent" :title="t('sidebar.changeTheme')">&#9728;</button>
      </div>
      <div class="p-3 flex flex-col gap-1.5 border-b border-border">
        <input id="new-project-input" :placeholder="t('project.projectName') + '…'" autocomplete="one-time-code" name="project-title-new"
          class="bg-surface-2 border border-border rounded-md text-text font-sans text-[13px] px-2.5 py-1.5 outline-none transition-colors focus:border-accent" />
        <button id="new-project-btn" @click="handleCreateProject"
          class="bg-accent-dim border border-accent rounded-md text-text cursor-pointer font-sans text-[13px] px-2.5 py-1.5 transition-colors hover:bg-accent">{{ t('create') }}</button>
      </div>
      <ul id="project-list" class="list-none flex-1 overflow-y-auto py-2"></ul>
      <!-- Language Switcher -->
      <div class="px-3 py-2 border-t border-border mt-auto">
        <select
          :value="locale"
          @change="setLocale(($event.target as HTMLSelectElement).value as any)"
          class="w-full bg-surface-2 border border-border rounded-md text-text-dim font-mono text-[11px] px-2 py-1 outline-none cursor-pointer focus:border-accent"
        >
          <option v-for="l in locales" :key="l.code" :value="l.code">{{ l.label }}</option>
        </select>
      </div>
      <div class="sidebar-user border-b-0 p-3 flex flex-col gap-2" id="sidebar-user">
        <div class="flex items-center gap-2">
          <span class="user-avatar bg-accent-dim border border-accent rounded-full text-accent inline-flex items-center justify-center font-mono text-xs font-semibold h-7 w-7 uppercase flex-shrink-0" id="user-avatar"></span>
          <div class="flex flex-col overflow-hidden">
            <span class="user-name text-[13px] font-semibold text-text overflow-hidden text-ellipsis whitespace-nowrap" id="user-name"></span>
            <span class="user-role text-[11px] text-text-dim font-mono uppercase" id="user-role"></span>
          </div>
        </div>
        <div class="flex gap-1">
          <button id="password-btn" class="bg-transparent border border-border rounded-md text-text-dim cursor-pointer text-xs px-2 py-1 transition-all flex-1 text-center hover:border-accent hover:text-accent" :title="t('sidebar.changePassword')">&#128273;</button>
          <button id="admin-btn" class="bg-transparent border border-border rounded-md text-text-dim cursor-pointer text-xs px-2 py-1 transition-all flex-1 text-center hover:border-accent hover:text-accent" :title="t('sidebar.admin')" style="display:none">&#9881;</button>
          <button id="logout-btn" class="bg-transparent border border-border rounded-md text-text-dim cursor-pointer text-xs px-2 py-1 transition-all flex-1 text-center hover:border-accent hover:text-accent" :title="t('sidebar.logout')">&#9211;</button>
        </div>
      </div>
    </aside>

    <!-- Sidebar Overlay (Mobile) -->
    <div class="sidebar-overlay hidden fixed inset-0 z-[999] bg-black/50" onclick="document.querySelector('.sidebar').classList.remove('sidebar-open')"></div>
    <!-- Hauptbereich -->
    <main class="flex-1 flex flex-col overflow-hidden">
      <header class="px-6 py-4 pb-3 border-b border-border bg-surface flex items-center gap-3 relative">
        <button class="sidebar-toggle hidden bg-transparent border border-border rounded-md text-text-dim text-base px-2 py-1 cursor-pointer flex-shrink-0 hover:border-accent hover:text-accent" onclick="document.querySelector('.sidebar').classList.toggle('sidebar-open')">&#9776;</button>
        <h1 id="project-title" class="font-mono text-base font-semibold tracking-tight flex-1"></h1>
        <span id="git-status-icon" class="git-status-icon cursor-pointer text-base ml-2 opacity-70 transition-opacity hover:opacity-100" style="display:none" title="Git"></span>
        <button class="bg-transparent border border-border rounded-md text-text-dim cursor-pointer text-sm px-2.5 py-1 transition-all ml-auto hover:border-accent hover:text-accent" :title="t('board.search') + ' (Ctrl+K)'" onclick="window.__kanbanToggleSearch?.()">&#128269;</button>
        <button id="import-btn" class="bg-transparent border border-border rounded-md text-text-dim cursor-pointer font-sans text-xs px-2.5 py-1 transition-all hover:border-accent hover:text-accent" :title="t('board.importIssues')">&#8615; {{ t('board.importIssues') }}</button>
        <button id="project-menu-btn" class="bg-transparent border border-border rounded-md text-text-dim cursor-pointer text-base px-2.5 py-1 transition-all ml-auto hover:border-accent hover:text-accent" :title="t('board.projectMenu')">&#9776;</button>
        <div id="project-dropdown" class="project-dropdown absolute top-full right-6 z-[2000] bg-surface border border-border rounded-md shadow-[0_8px_24px_rgba(0,0,0,0.4)] py-1 min-w-[200px]"></div>
      </header>
      <div id="bulk-bar" class="bulk-bar items-center gap-3 px-6 py-2 bg-surface border-b border-accent text-[13px] text-text">
        <span v-html="t('bulk.selected', { count: '<strong id=\'bulk-count\'>0</strong>' })"></span>
        <button id="bulk-delete-btn" class="bg-transparent border border-danger text-danger rounded-md cursor-pointer text-xs px-2 py-0.5 hover:bg-danger/10">{{ t('bulk.deleteSelected') }}</button>
        <button id="bulk-cancel-btn" class="bg-accent-dim border border-accent rounded-md text-text cursor-pointer text-sm px-2 py-0.5 transition-colors hover:bg-accent">{{ t('bulk.deselectAll') }}</button>
      </div>
      <div id="board" class="flex-1 overflow-x-auto overflow-y-hidden p-5 px-6">
        <KanbanBoard />
      </div>
    </main>
  </div>

  <!-- Vue-Komponenten für Task-Modal und Task-Detail -->
  <TaskModal ref="taskModalRef" />
  <TaskDetail ref="taskDetailRef" @edit="onEditFromDetail" />

  <!-- Spalten-Modal (Legacy) -->
  <div id="column-modal" class="modal-overlay fixed inset-0 bg-black/70 backdrop-blur-[2px] z-[1000] items-center justify-center">
    <div class="bg-surface border border-border rounded-lg shadow-[0_16px_48px_rgba(0,0,0,0.5)] flex flex-col gap-3.5 max-w-[480px] p-6 w-[90%]">
      <div class="flex items-center justify-between">
        <span class="font-mono text-[13px] font-semibold tracking-wide uppercase text-text-dim" id="col-modal-heading">{{ t('column.column') }}</span>
        <button class="bg-transparent border-none text-text-dim cursor-pointer text-base px-1.5 py-0.5 hover:text-text" id="col-modal-close">&#10005;</button>
      </div>
      <label class="flex flex-col gap-1.5 text-xs text-text-dim font-mono uppercase tracking-wide">{{ t('column.title') }}
        <input id="col-modal-title" type="text" :placeholder="t('column.title') + '…'"
          class="bg-surface-2 border border-border rounded-md text-text font-sans text-sm px-3 py-2 outline-none transition-colors focus:border-accent" />
      </label>
      <div class="flex flex-col gap-2">
        <span class="font-mono text-xs text-text-dim uppercase tracking-wide">{{ t('column.color') }}</span>
        <div id="col-modal-colors" class="color-grid"></div>
      </div>
      <div class="flex gap-2 justify-end mt-1">
        <button id="col-modal-save" class="bg-accent border-none text-white font-semibold rounded-md px-5 py-2 text-[13px] cursor-pointer hover:opacity-85 transition-opacity">{{ t('save') }}</button>
      </div>
    </div>
  </div>

  <!-- Projekt-Modal (Legacy) -->
  <div id="project-modal" class="modal-overlay fixed inset-0 bg-black/70 backdrop-blur-[2px] z-[1000] items-center justify-center">
    <div class="bg-surface border border-border rounded-lg shadow-[0_16px_48px_rgba(0,0,0,0.5)] flex flex-col gap-3.5 max-w-[1000px] p-6 w-[90%]">
      <div class="flex items-center justify-between">
        <span class="font-mono text-[13px] font-semibold tracking-wide uppercase text-text-dim">{{ t('project.project') }}</span>
        <button class="bg-transparent border-none text-text-dim cursor-pointer text-base px-1.5 py-0.5 hover:text-text" id="proj-modal-close">&#10005;</button>
      </div>
      <label class="flex flex-col gap-1.5 text-xs text-text-dim font-mono uppercase tracking-wide">{{ t('project.projectName') }}
        <input id="proj-modal-title" type="text" :placeholder="t('project.projectName') + '…'" autocomplete="one-time-code" name="project-title-edit"
          class="bg-surface-2 border border-border rounded-md text-text font-sans text-sm px-3 py-2 outline-none transition-colors focus:border-accent" />
      </label>
      <div class="flex items-center justify-between">
        <span class="font-mono text-[10px] text-text-dim uppercase tracking-wide">JSON</span>
        <button id="proj-view-toggle" class="bg-accent-dim border border-accent rounded-md text-text cursor-pointer text-sm px-2 py-0.5 transition-colors hover:bg-accent">{{ t('project.rawJson') }}</button>
      </div>
      <div id="proj-json-tree" class="json-tree"></div>
      <textarea id="proj-modal-json" class="bg-surface-2 border border-border rounded-md text-text font-mono text-xs leading-relaxed p-3 resize-y w-full outline-none transition-colors focus:border-accent" rows="20" spellcheck="false" style="display:none"></textarea>
      <div class="flex gap-2 justify-end mt-1">
        <button id="proj-modal-copy" class="bg-accent-dim border border-accent rounded-md text-text cursor-pointer text-sm px-2 py-0.5 transition-colors hover:bg-accent">{{ t('copyToClipboard') }}</button>
        <button id="proj-modal-save" class="bg-accent border-none text-white font-semibold rounded-md px-5 py-2 text-[13px] cursor-pointer hover:opacity-85 transition-opacity">{{ t('save') }}</button>
        <button id="proj-modal-import" class="bg-accent-dim border border-accent rounded-md text-text cursor-pointer text-sm px-2 py-0.5 transition-colors hover:bg-accent">{{ t('project.importAsNew') }}</button>
      </div>
    </div>
  </div>

  <!-- Git-Modal (Legacy) -->
  <div id="git-modal" class="modal-overlay fixed inset-0 bg-black/70 backdrop-blur-[2px] z-[1000] items-center justify-center">
    <div class="bg-surface border border-border rounded-lg shadow-[0_16px_48px_rgba(0,0,0,0.5)] flex flex-col gap-3.5 max-w-[480px] p-6 w-[90%]">
      <div class="flex items-center justify-between">
        <span class="font-mono text-[13px] font-semibold tracking-wide uppercase text-text-dim">{{ t('git.settings') }}</span>
        <button class="bg-transparent border-none text-text-dim cursor-pointer text-base px-1.5 py-0.5 hover:text-text" id="git-modal-close">&#10005;</button>
      </div>
      <label class="flex flex-col gap-1.5 text-xs text-text-dim font-mono uppercase tracking-wide">{{ t('git.repoUrl') }}
        <input id="git-repo-url" type="text" placeholder="https://token:ghp_xxx@github.com/user/repo.git"
          class="bg-surface-2 border border-border rounded-md text-text font-sans text-sm px-3 py-2 outline-none transition-colors focus:border-accent" />
      </label>
      <label class="flex flex-col gap-1.5 text-xs text-text-dim font-mono uppercase tracking-wide">{{ t('git.branch') }}
        <input id="git-branch" type="text" placeholder="main"
          class="bg-surface-2 border border-border rounded-md text-text font-sans text-sm px-3 py-2 outline-none transition-colors focus:border-accent" />
      </label>
      <label class="flex flex-col gap-1.5 text-xs text-text-dim font-mono uppercase tracking-wide">{{ t('git.pathInRepo') }}
        <input id="git-path" type="text" placeholder="plankton.json"
          class="bg-surface-2 border border-border rounded-md text-text font-sans text-sm px-3 py-2 outline-none transition-colors focus:border-accent" />
      </label>
      <label class="flex items-center gap-2 cursor-pointer text-[13px] my-2">
        <input id="git-enabled" type="checkbox" class="w-4 h-4 accent-accent" />
        {{ t('git.autoSync') }}
      </label>
      <div id="git-status" class="text-xs my-3"></div>
      <div class="flex gap-2 justify-end mt-1">
        <button id="git-sync-btn" class="bg-accent-dim border border-accent rounded-md text-text cursor-pointer text-sm px-2 py-0.5 transition-colors hover:bg-accent">{{ t('git.syncNow') }}</button>
        <button id="git-save-btn" class="bg-accent border-none text-white font-semibold rounded-md px-5 py-2 text-[13px] cursor-pointer hover:opacity-85 transition-opacity">{{ t('save') }}</button>
      </div>
    </div>
  </div>

  <!-- Prompt-Modal (Legacy) -->
  <div id="prompt-modal" class="modal-overlay fixed inset-0 bg-black/70 backdrop-blur-[2px] z-[1000] items-center justify-center">
    <div class="bg-surface border border-border rounded-lg shadow-[0_16px_48px_rgba(0,0,0,0.5)] flex flex-col gap-3.5 max-w-[1000px] p-6 w-[90%]">
      <div class="flex items-center justify-between">
        <span class="font-mono text-[13px] font-semibold tracking-wide uppercase text-text-dim">{{ t('prompt.aiAgents') }}</span>
        <button class="bg-transparent border-none text-text-dim cursor-pointer text-base px-1.5 py-0.5 hover:text-text" id="prompt-modal-close">&#10005;</button>
      </div>
      <!-- Tab-Leiste -->
      <div class="prompt-tabs flex gap-1 border-b border-border pb-2">
        <button class="prompt-tab prompt-tab-active bg-transparent border border-border rounded-t-md text-text-dim cursor-pointer font-mono text-xs px-3.5 py-1.5 uppercase tracking-wide transition-all hover:text-text" data-prompt-tab="simple">{{ t('prompt.simple') }}</button>
        <button class="prompt-tab bg-transparent border border-border rounded-t-md text-text-dim cursor-pointer font-mono text-xs px-3.5 py-1.5 uppercase tracking-wide transition-all hover:text-text" data-prompt-tab="claudeai">claude.ai</button>
        <button class="prompt-tab bg-transparent border border-border rounded-t-md text-text-dim cursor-pointer font-mono text-xs px-3.5 py-1.5 uppercase tracking-wide transition-all hover:text-text" data-prompt-tab="plankton">Plankton</button>
      </div>
      <!-- Tab: Simple -->
      <div id="prompt-tab-simple" class="prompt-tab-content prompt-tab-visible">
        <pre id="prompt-content" class="bg-surface-2 border border-border rounded-md text-text font-mono text-xs leading-relaxed p-4 max-h-[500px] overflow-y-auto whitespace-pre-wrap break-words select-all"></pre>
        <div class="flex gap-2 justify-end mt-1">
          <button id="prompt-copy-btn" class="bg-accent border-none text-white font-semibold rounded-md px-5 py-2 text-[13px] cursor-pointer hover:opacity-85 transition-opacity">{{ t('copyToClipboard') }}</button>
        </div>
      </div>
      <!-- Tab: claude.ai -->
      <div id="prompt-tab-claudeai" class="prompt-tab-content">
        <div class="flex flex-col gap-2 py-1">
          <h3 class="m-0 text-base text-text">{{ t('prompt.connectorTitle') }}</h3>
          <p class="m-0 text-[13px] text-text-dim leading-relaxed" v-html="t('prompt.connectorDesc')"></p>
          <h3 class="mt-3 mb-1 text-[13px] text-accent">{{ t('prompt.connectorStep1') }}</h3>
          <p class="m-0 text-[13px] text-text-dim leading-relaxed" v-html="t('prompt.connectorStep1Desc')"></p>
          <div class="flex flex-col gap-1.5 my-3">
            <div class="flex items-center gap-3 px-2.5 py-1.5 bg-surface-2 border border-border rounded-md">
              <span class="font-mono text-[11px] text-text-dim uppercase tracking-wide min-w-[140px] flex-shrink-0">Server URL</span>
              <code id="claudeai-server-url" class="font-mono text-xs text-accent break-all">...</code>
            </div>
          </div>
          <p class="text-xs text-text-dim m-0" v-html="t('prompt.connectorOAuthNote')"></p>
          <h3 class="mt-3 mb-1 text-[13px] text-accent">{{ t('prompt.connectorStep2') }}</h3>
          <p class="m-0 text-[13px] text-text-dim leading-relaxed" v-html="t('prompt.connectorStep2Desc')"></p>
          <p class="text-xs text-text-dim m-0" v-html="t('prompt.connectorOAuthDetail')"></p>
        </div>
      </div>
      <!-- Tab: Plankton -->
      <div id="prompt-tab-plankton" class="prompt-tab-content">
        <div class="flex flex-col gap-3.5">
          <label class="flex flex-col gap-1.5 text-xs text-text-dim font-mono uppercase tracking-wide">Plankton-URL
            <input id="prompt-plankton-url" type="text" placeholder="https://plankton.example.com"
              class="bg-surface-2 border border-border rounded-md text-text font-sans text-sm px-3 py-2 outline-none transition-colors focus:border-accent" />
          </label>
          <div class="flex flex-col gap-2">
            <span class="font-mono text-[10px] text-text-dim uppercase tracking-wide">Agent-Tokens</span>
            <p class="text-xs text-text-dim m-0" v-html="t('prompt.tokensNote')"></p>
            <div id="prompt-token-list" class="flex flex-col gap-1.5 max-h-[200px] overflow-y-auto"></div>
            <div id="prompt-token-loading" class="text-xs text-text-dim">{{ t('prompt.loadingTokens') }}</div>
          </div>
          <div class="flex gap-2 justify-end mt-1">
            <button id="prompt-generate-btn" class="bg-accent border-none text-white font-semibold rounded-md px-5 py-2 text-[13px] cursor-pointer hover:opacity-85 transition-opacity">{{ t('prompt.generateFiles') }}</button>
          </div>
        </div>
        <div id="prompt-output" class="flex flex-col gap-2.5 mt-2" style="display:none">
          <div class="prompt-output-tabs flex gap-1 border-b border-border pb-1.5">
            <button class="prompt-output-tab prompt-output-tab-active bg-transparent border border-border rounded-t-md text-text-dim cursor-pointer font-mono text-[11px] px-2.5 py-1 transition-all hover:text-text" data-output-tab="setup">Claude Code Setup</button>
            <button class="prompt-output-tab bg-transparent border border-border rounded-t-md text-text-dim cursor-pointer font-mono text-[11px] px-2.5 py-1 transition-all hover:text-text" data-output-tab="rules">rules.md</button>
            <button class="prompt-output-tab bg-transparent border border-border rounded-t-md text-text-dim cursor-pointer font-mono text-[11px] px-2.5 py-1 transition-all hover:text-text" data-output-tab="workflow">workflow.md</button>
          </div>
          <div id="prompt-out-setup" class="prompt-output-content prompt-tab-visible">
            <div>
              <p class="text-[13px] text-text-dim" v-html="t('prompt.installSkillDesc')"></p>
              <pre class="bg-surface-2 border border-border rounded-md text-text font-mono text-xs leading-relaxed p-4 max-h-[500px] overflow-y-auto whitespace-pre-wrap break-words select-all" id="prompt-out-setup-pre"></pre>
              <p class="text-xs text-text-dim" v-html="t('prompt.installSkillNote')"></p>
            </div>
          </div>
          <div id="prompt-out-rules" class="prompt-output-content">
            <pre class="bg-surface-2 border border-border rounded-md text-text font-mono text-xs leading-relaxed p-4 max-h-[500px] overflow-y-auto whitespace-pre-wrap break-words select-all" id="prompt-out-rules-pre"></pre>
          </div>
          <div id="prompt-out-workflow" class="prompt-output-content">
            <pre class="bg-surface-2 border border-border rounded-md text-text font-mono text-xs leading-relaxed p-4 max-h-[500px] overflow-y-auto whitespace-pre-wrap break-words select-all" id="prompt-out-workflow-pre"></pre>
          </div>
          <div class="flex gap-2 justify-end mt-1">
            <button id="prompt-out-copy" class="bg-accent border-none text-white font-semibold rounded-md px-5 py-2 text-[13px] cursor-pointer hover:opacity-85 transition-opacity">{{ t('copyToClipboard') }}</button>
            <button id="prompt-out-download" class="bg-accent-dim border border-accent rounded-md text-text cursor-pointer text-sm px-2 py-0.5 transition-colors hover:bg-accent">&#8615; {{ t('download') }}</button>
          </div>
        </div>
      </div>
    </div>
  </div>

  <!-- CLI-Modal (Install CLI) -->
  <div id="cli-modal" class="modal-overlay fixed inset-0 bg-black/70 backdrop-blur-[2px] z-[1000] items-center justify-center">
    <div class="bg-surface border border-border rounded-lg shadow-[0_16px_48px_rgba(0,0,0,0.5)] flex flex-col gap-3.5 max-w-[1000px] p-6 w-[90%]">
      <div class="flex items-center justify-between">
        <span class="font-mono text-[13px] font-semibold tracking-wide uppercase text-text-dim">{{ t('prompt.installCli') }}</span>
        <button class="bg-transparent border-none text-text-dim cursor-pointer text-base px-1.5 py-0.5 hover:text-text" id="cli-modal-close">&#10005;</button>
      </div>
      <div class="flex flex-col gap-2 py-1">
        <h3 class="m-0 text-base text-text">{{ t('prompt.installation') }}</h3>
        <p class="m-0 text-[13px] text-text-dim leading-relaxed" v-html="t('prompt.installCliDesc')"></p>
        <pre class="bg-surface-2 border border-border rounded-md text-text font-mono text-xs leading-relaxed p-4 max-h-[500px] overflow-y-auto whitespace-pre-wrap break-words select-all" id="cli-install-cmd">curl -fsSL .../install | bash</pre>
        <div class="flex gap-2 justify-end mt-1">
          <button class="bg-accent-dim border border-accent rounded-md text-text cursor-pointer text-sm px-2 py-0.5 transition-colors hover:bg-accent" data-cli-copy="cli-install-cmd">{{ t('copyToClipboard') }}</button>
        </div>

        <h3 class="mt-3 mb-1 text-base text-text">{{ t('prompt.loginTitle') }}</h3>
        <p class="m-0 text-[13px] text-text-dim leading-relaxed" v-html="t('prompt.loginDesc')"></p>
        <pre class="bg-surface-2 border border-border rounded-md text-text font-mono text-xs leading-relaxed p-4 max-h-[500px] overflow-y-auto whitespace-pre-wrap break-words select-all" id="cli-login-cmd">plankton remote add origin ...</pre>
        <div class="flex gap-2 justify-end mt-1">
          <button class="bg-accent-dim border border-accent rounded-md text-text cursor-pointer text-sm px-2 py-0.5 transition-colors hover:bg-accent" data-cli-copy="cli-login-cmd">{{ t('copyToClipboard') }}</button>
        </div>

        <h3 class="mt-3 mb-1 text-base text-text">{{ t('prompt.claudeCodeSkill') }}</h3>
        <p class="m-0 text-[13px] text-text-dim leading-relaxed" v-html="t('prompt.claudeCodeSkillDesc')"></p>
        <pre class="bg-surface-2 border border-border rounded-md text-text font-mono text-xs leading-relaxed p-4 max-h-[500px] overflow-y-auto whitespace-pre-wrap break-words select-all" id="cli-skill-cmd">plankton skill install ... --global</pre>
        <div class="flex gap-2 justify-end mt-1">
          <button class="bg-accent-dim border border-accent rounded-md text-text cursor-pointer text-sm px-2 py-0.5 transition-colors hover:bg-accent" data-cli-copy="cli-skill-cmd">{{ t('copyToClipboard') }}</button>
        </div>

        <h3 class="mt-3 mb-1 text-base text-text">{{ t('prompt.help') }}</h3>
        <pre class="bg-surface-2 border border-border rounded-md text-text font-mono text-xs leading-relaxed p-4 max-h-[500px] overflow-y-auto whitespace-pre-wrap break-words select-all">plankton help                    # Alle Befehle anzeigen
plankton remote add origin ...   # Login + Update: gleicher Befehl
curl -fsSL .../install | bash    # CLI aktualisieren</pre>
      </div>
    </div>
  </div>

  <!-- Admin-Modal (Legacy) -->
  <div id="admin-modal" class="modal-overlay fixed inset-0 bg-black/70 backdrop-blur-[2px] z-[1000] items-center justify-center">
    <div class="bg-surface border border-border rounded-lg shadow-[0_16px_48px_rgba(0,0,0,0.5)] flex flex-col gap-3.5 max-w-[1000px] p-6 w-[90%]">
      <div class="flex items-center justify-between">
        <span class="font-mono text-[13px] font-semibold tracking-wide uppercase text-text-dim">{{ t('admin.administration') }}</span>
        <button class="bg-transparent border-none text-text-dim cursor-pointer text-base px-1.5 py-0.5 hover:text-text" id="admin-modal-close">&#10005;</button>
      </div>
      <div class="admin-tabs flex gap-1 border-b border-border pb-2">
        <button class="admin-tab admin-tab-active bg-transparent border border-border rounded-t-md text-text-dim cursor-pointer font-mono text-xs px-3.5 py-1.5 uppercase tracking-wide transition-all hover:text-text" data-tab="users">{{ t('admin.users') }}</button>
        <button class="admin-tab bg-transparent border border-border rounded-t-md text-text-dim cursor-pointer font-mono text-xs px-3.5 py-1.5 uppercase tracking-wide transition-all hover:text-text" data-tab="tokens">{{ t('admin.tokens') }}</button>
      </div>
      <div id="admin-user-list" class="admin-user-list flex flex-col gap-1.5 max-h-[400px] overflow-y-auto"></div>
      <div id="admin-user-form" class="flex flex-col gap-3" style="display:none">
        <label class="flex flex-col gap-1.5 text-xs text-text-dim font-mono uppercase tracking-wide">{{ t('admin.username') }} <input id="admin-username" type="text" class="bg-surface-2 border border-border rounded-md text-text font-sans text-sm px-3 py-2 outline-none transition-colors focus:border-accent" /></label>
        <label class="flex flex-col gap-1.5 text-xs text-text-dim font-mono uppercase tracking-wide">{{ t('admin.displayName') }} <input id="admin-displayname" type="text" class="bg-surface-2 border border-border rounded-md text-text font-sans text-sm px-3 py-2 outline-none transition-colors focus:border-accent" /></label>
        <label class="flex flex-col gap-1.5 text-xs text-text-dim font-mono uppercase tracking-wide">{{ t('admin.password') }} <input id="admin-password" type="password" class="bg-surface-2 border border-border rounded-md text-text font-sans text-sm px-3 py-2 outline-none transition-colors focus:border-accent" /></label>
        <label class="flex flex-col gap-1.5 text-xs text-text-dim font-mono uppercase tracking-wide">{{ t('admin.role') }}
          <select id="admin-role">
            <option value="user">User</option>
            <option value="admin">Admin</option>
          </select>
        </label>
        <div class="flex gap-2 justify-end mt-1">
          <button id="admin-form-save" class="bg-accent border-none text-white font-semibold rounded-md px-5 py-2 text-[13px] cursor-pointer hover:opacity-85 transition-opacity">{{ t('save') }}</button>
          <button id="admin-form-cancel" class="bg-accent-dim border border-accent rounded-md text-text cursor-pointer text-sm px-2 py-0.5 transition-colors hover:bg-accent">{{ t('cancel') }}</button>
        </div>
      </div>
      <div class="flex gap-2 justify-end mt-1" id="admin-list-actions">
        <button id="admin-add-user-btn" class="bg-accent border-none text-white font-semibold rounded-md px-5 py-2 text-[13px] cursor-pointer hover:opacity-85 transition-opacity">{{ t('admin.newUser') }}</button>
      </div>
      <div id="admin-token-section" style="display:none">
        <div id="admin-token-list" class="admin-user-list flex flex-col gap-1.5 max-h-[400px] overflow-y-auto"></div>
        <div class="flex gap-2 items-center mt-3">
          <input id="admin-token-name" type="text" :placeholder="t('admin.tokenName')" class="flex-1 bg-surface-2 border border-border rounded-md text-text text-[13px] px-2.5 py-1.5 outline-none focus:border-accent" />
          <select id="admin-token-role" class="bg-surface-2 border border-border rounded-md text-text text-[13px] px-2.5 py-1.5 outline-none">
            <option value="developer">Developer</option>
            <option value="tester">Tester</option>
            <option value="manager">Manager</option>
          </select>
          <button id="admin-create-token-btn" class="bg-accent border-none text-white font-semibold rounded-md px-5 py-2 text-[13px] cursor-pointer hover:opacity-85 transition-opacity">{{ t('create') }} Token</button>
        </div>
        <pre id="admin-token-result" class="bg-surface-2 border border-accent rounded-md text-accent font-mono text-xs p-2.5 break-all mt-2 select-all" style="display:none"></pre>
      </div>
    </div>
  </div>

  <!-- Passwort-Modal (Legacy) -->
  <div id="password-modal" class="modal-overlay fixed inset-0 bg-black/70 backdrop-blur-[2px] z-[1000] items-center justify-center">
    <div class="bg-surface border border-border rounded-lg shadow-[0_16px_48px_rgba(0,0,0,0.5)] flex flex-col gap-3.5 max-w-[480px] p-6 w-[90%]">
      <div class="flex items-center justify-between">
        <span class="font-mono text-[13px] font-semibold tracking-wide uppercase text-text-dim">{{ t('passwordModal.changePassword') }}</span>
        <button class="bg-transparent border-none text-text-dim cursor-pointer text-base px-1.5 py-0.5 hover:text-text" id="pw-modal-close">&#10005;</button>
      </div>
      <div id="pw-error" class="text-[#ff6b6b] text-[13px]"></div>
      <label class="flex flex-col gap-1.5 text-xs text-text-dim font-mono uppercase tracking-wide">{{ t('passwordModal.oldPassword') }} <input id="pw-old" type="password" class="bg-surface-2 border border-border rounded-md text-text font-sans text-sm px-3 py-2 outline-none transition-colors focus:border-accent" /></label>
      <label class="flex flex-col gap-1.5 text-xs text-text-dim font-mono uppercase tracking-wide">{{ t('passwordModal.newPassword') }} <input id="pw-new" type="password" class="bg-surface-2 border border-border rounded-md text-text font-sans text-sm px-3 py-2 outline-none transition-colors focus:border-accent" /></label>
      <label class="flex flex-col gap-1.5 text-xs text-text-dim font-mono uppercase tracking-wide">{{ t('passwordModal.confirmPassword') }} <input id="pw-confirm" type="password" class="bg-surface-2 border border-border rounded-md text-text font-sans text-sm px-3 py-2 outline-none transition-colors focus:border-accent" /></label>
      <div class="flex gap-2 justify-end mt-1">
        <button id="pw-save-btn" class="bg-accent border-none text-white font-semibold rounded-md px-5 py-2 text-[13px] cursor-pointer hover:opacity-85 transition-opacity">{{ t('save') }}</button>
      </div>
    </div>
  </div>

  <!-- Import-Modal (Legacy) -->
  <div id="import-modal" class="modal-overlay fixed inset-0 bg-black/70 backdrop-blur-[2px] z-[1000] items-center justify-center">
    <div class="bg-surface border border-border rounded-lg shadow-[0_16px_48px_rgba(0,0,0,0.5)] flex flex-col gap-3.5 max-w-[1000px] p-6 w-[90%]">
      <div class="flex items-center justify-between">
        <span class="font-mono text-[13px] font-semibold tracking-wide uppercase text-text-dim">{{ t('import.importIssues') }}</span>
        <button class="bg-transparent border-none text-text-dim cursor-pointer text-base px-1.5 py-0.5 hover:text-text" id="import-modal-close">&#10005;</button>
      </div>
      <label class="flex flex-col gap-1.5 text-xs text-text-dim font-mono uppercase tracking-wide">{{ t('import.jsonLabel') }}
        <textarea id="import-json" rows="10" placeholder='[{"title": "...", "column_slug": "TODO", "points": 3, "labels": ["feature"]}]' spellcheck="false"
          class="bg-surface-2 border border-border rounded-md text-text font-sans text-sm px-3 py-2 outline-none resize-y transition-colors focus:border-accent"></textarea>
      </label>
      <div class="flex gap-2 justify-end mt-1">
        <button id="import-validate-btn" class="bg-accent-dim border border-accent rounded-md text-text cursor-pointer text-sm px-2 py-0.5 transition-colors hover:bg-accent">{{ t('import.validate') }}</button>
        <button id="import-start-btn" class="bg-accent border-none text-white font-semibold rounded-md px-5 py-2 text-[13px] cursor-pointer hover:opacity-85 transition-opacity" style="display:none">{{ t('import.startImport') }}</button>
      </div>
      <div id="import-preview" class="max-h-[180px] overflow-y-auto" style="display:none"></div>
      <div id="import-result" class="py-2" style="display:none"></div>
    </div>
  </div>
</template>
