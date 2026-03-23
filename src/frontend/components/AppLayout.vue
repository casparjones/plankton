<script setup lang="ts">
// Haupt-Layout der Plankton-App: Sidebar, Header, Board und alle Modals.
// Task-Modal und Task-Detail sind Vue-Komponenten, die übrigen Modals
// nutzen weiterhin Legacy-DOM mit Event-Listenern in onMounted().

import { ref, onMounted } from 'vue'
import KanbanBoard from './KanbanBoard.vue'
import TaskModal from './TaskModal.vue'
import TaskDetail from './TaskDetail.vue'
import type { Task } from '../types'

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
  <div class="app">
    <!-- Sidebar -->
    <aside class="sidebar">
      <div class="sidebar-header">
        <span class="logo"><img src="/icons/logo.svg" alt="" class="logo-icon" /> Plankton</span>
        <button id="theme-toggle" class="theme-toggle" title="Theme wechseln">&#9728;</button>
      </div>
      <div class="sidebar-create">
        <input id="new-project-input" placeholder="Projektname…" autocomplete="one-time-code" name="project-title-new" />
        <button id="new-project-btn" @click="handleCreateProject">Erstellen</button>
      </div>
      <ul id="project-list" class="project-list"></ul>
      <div class="sidebar-user" id="sidebar-user">
        <div class="user-info">
          <span class="user-avatar" id="user-avatar"></span>
          <div class="user-details">
            <span class="user-name" id="user-name"></span>
            <span class="user-role" id="user-role"></span>
          </div>
        </div>
        <div class="user-actions">
          <button id="password-btn" class="user-action-btn" title="Passwort ändern">&#128273;</button>
          <button id="admin-btn" class="user-action-btn" title="Admin" style="display:none">&#9881;</button>
          <button id="logout-btn" class="user-action-btn" title="Abmelden">&#9211;</button>
        </div>
      </div>
    </aside>

    <!-- Sidebar Overlay (Mobile) -->
    <div class="sidebar-overlay" onclick="document.querySelector('.sidebar').classList.remove('sidebar-open')"></div>
    <!-- Hauptbereich -->
    <main class="main">
      <header class="board-header">
        <button class="sidebar-toggle" onclick="document.querySelector('.sidebar').classList.toggle('sidebar-open')">&#9776;</button>
        <h1 id="project-title" class="board-title"></h1>
        <span id="git-status-icon" class="git-status-icon" style="display:none" title="Git"></span>
        <button class="search-toggle-btn" title="Suche (Ctrl+K)" onclick="window.__kanbanToggleSearch?.()">&#128269;</button>
        <button id="import-btn" class="import-btn" title="Issues importieren">&#8615; Import</button>
        <button id="project-menu-btn" class="project-menu-btn" title="Projekt-Menü">&#9776;</button>
        <div id="project-dropdown" class="project-dropdown"></div>
      </header>
      <div id="bulk-bar" class="bulk-bar">
        <span><strong id="bulk-count">0</strong> Task(s) ausgewählt</span>
        <button id="bulk-delete-btn" class="btn-danger btn-small">Ausgewählte löschen</button>
        <button id="bulk-cancel-btn" class="btn-small">Auswahl aufheben</button>
      </div>
      <div id="board" class="board">
        <KanbanBoard />
      </div>
    </main>
  </div>

  <!-- Vue-Komponenten für Task-Modal und Task-Detail -->
  <TaskModal ref="taskModalRef" />
  <TaskDetail ref="taskDetailRef" @edit="onEditFromDetail" />

  <!-- Spalten-Modal (Legacy) -->
  <div id="column-modal" class="modal-overlay">
    <div class="modal">
      <div class="modal-header">
        <span class="modal-heading" id="col-modal-heading">Spalte</span>
        <button class="modal-close" id="col-modal-close">&#10005;</button>
      </div>
      <label>Titel
        <input id="col-modal-title" type="text" placeholder="Spaltenname…" />
      </label>
      <div class="color-picker-section">
        <span class="color-picker-label">Farbe</span>
        <div id="col-modal-colors" class="color-grid"></div>
      </div>
      <div class="modal-actions">
        <button id="col-modal-save" class="btn-primary">Speichern</button>
      </div>
    </div>
  </div>

  <!-- Projekt-Modal (Legacy) -->
  <div id="project-modal" class="modal-overlay">
    <div class="modal modal-wide">
      <div class="modal-header">
        <span class="modal-heading">Projekt</span>
        <button class="modal-close" id="proj-modal-close">&#10005;</button>
      </div>
      <label>Projektname
        <input id="proj-modal-title" type="text" placeholder="Projektname…" autocomplete="one-time-code" name="project-title-edit" />
      </label>
      <div class="proj-json-header">
        <span class="modal-section-title">JSON</span>
        <button id="proj-view-toggle" class="btn-small">Raw JSON</button>
      </div>
      <div id="proj-json-tree" class="json-tree"></div>
      <textarea id="proj-modal-json" class="proj-json-textarea" rows="20" spellcheck="false" style="display:none"></textarea>
      <div class="modal-actions">
        <button id="proj-modal-copy" class="btn-small">In Zwischenablage kopieren</button>
        <button id="proj-modal-save" class="btn-primary">Speichern</button>
        <button id="proj-modal-import" class="btn-small">Als neues Projekt importieren</button>
      </div>
    </div>
  </div>

  <!-- Git-Modal (Legacy) -->
  <div id="git-modal" class="modal-overlay">
    <div class="modal">
      <div class="modal-header">
        <span class="modal-heading">Git-Einstellungen</span>
        <button class="modal-close" id="git-modal-close">&#10005;</button>
      </div>
      <label>Repository-URL
        <input id="git-repo-url" type="text" placeholder="https://token:ghp_xxx@github.com/user/repo.git" />
      </label>
      <label>Branch
        <input id="git-branch" type="text" placeholder="main" />
      </label>
      <label>Pfad im Repository
        <input id="git-path" type="text" placeholder="plankton.json" />
      </label>
      <label class="git-toggle-label">
        <input id="git-enabled" type="checkbox" />
        Auto-Sync aktiviert
      </label>
      <div id="git-status" class="git-status"></div>
      <div class="modal-actions">
        <button id="git-sync-btn" class="btn-small">Jetzt synchronisieren</button>
        <button id="git-save-btn" class="btn-primary">Speichern</button>
      </div>
    </div>
  </div>

  <!-- Prompt-Modal (Legacy) -->
  <div id="prompt-modal" class="modal-overlay">
    <div class="modal modal-wide">
      <div class="modal-header">
        <span class="modal-heading">AI Agents</span>
        <button class="modal-close" id="prompt-modal-close">&#10005;</button>
      </div>
      <!-- Tab-Leiste -->
      <div class="prompt-tabs">
        <button class="prompt-tab prompt-tab-active" data-prompt-tab="simple">Simple</button>
        <button class="prompt-tab" data-prompt-tab="claudeai">claude.ai</button>
        <button class="prompt-tab" data-prompt-tab="plankton">Plankton</button>
      </div>
      <!-- Tab: Simple (bisheriger Prompt) -->
      <div id="prompt-tab-simple" class="prompt-tab-content prompt-tab-visible">
        <pre id="prompt-content" class="prompt-content"></pre>
        <div class="modal-actions">
          <button id="prompt-copy-btn" class="btn-primary">In Zwischenablage kopieren</button>
        </div>
      </div>
      <!-- Tab: claude.ai (Connector Setup) -->
      <div id="prompt-tab-claudeai" class="prompt-tab-content">
        <div class="prompt-skill-info">
          <h3>Plankton als Connector in claude.ai</h3>
          <p>Plankton l&auml;sst sich als benutzerdefinierter MCP-Connector in claude.ai einbinden. Damit kann Claude direkt auf das Kanban-Board zugreifen.</p>

          <h3>1. OAuth Client erstellen</h3>
          <p>Erstelle einen OAuth Client unter <strong>Admin &#9881; &rarr; Tokens</strong> oder per API:</p>
          <pre class="prompt-content" id="claudeai-create-client">POST /api/admin/oauth-clients
{
  "name": "claude.ai",
  "redirect_uris": ["https://claude.ai/oauth/callback"]
}</pre>
          <p class="prompt-token-hint">Notiere <code>client_id</code> und <code>client_secret</code> &ndash; sie werden nur einmalig angezeigt.</p>

          <h3>2. Connector in claude.ai hinzuf&uuml;gen</h3>
          <p>In claude.ai unter <strong>Settings &rarr; Integrations &rarr; Add MCP Connector</strong>:</p>
          <div class="claudeai-config">
            <div class="config-row"><span class="config-label">Server URL</span><code id="claudeai-server-url" class="config-value">...</code></div>
            <div class="config-row"><span class="config-label">Authorization URL</span><code id="claudeai-auth-url" class="config-value">...</code></div>
            <div class="config-row"><span class="config-label">Token URL</span><code id="claudeai-token-url" class="config-value">...</code></div>
            <div class="config-row"><span class="config-label">Client ID</span><code class="config-value">(aus Schritt 1)</code></div>
            <div class="config-row"><span class="config-label">Client Secret</span><code class="config-value">(aus Schritt 1)</code></div>
          </div>

          <h3>3. Autorisieren</h3>
          <p>Beim ersten Zugriff &ouml;ffnet claude.ai ein Login-Fenster. Melde dich mit deinem Plankton-Account an &ndash; fertig.</p>
          <p class="prompt-token-hint">OAuth 2.0 Authorization Code Flow mit PKCE und Refresh Token Rotation.</p>
        </div>
      </div>
      <!-- Tab: Plankton (Agenten-Workflow-Generator) -->
      <div id="prompt-tab-plankton" class="prompt-tab-content">
        <div class="prompt-plankton-config">
          <label>Plankton-URL
            <input id="prompt-plankton-url" type="text" placeholder="https://plankton.example.com" />
          </label>
          <div class="prompt-token-section">
            <span class="modal-section-title">Agent-Tokens</span>
            <p class="prompt-token-hint">Tokens können unter <strong>Admin (&#9881;) → Tokens</strong> verwaltet werden.</p>
            <div id="prompt-token-list" class="prompt-token-list"></div>
            <div id="prompt-token-loading" class="prompt-token-hint">Lade Tokens...</div>
          </div>
          <div class="modal-actions">
            <button id="prompt-generate-btn" class="btn-primary">Dateien generieren</button>
          </div>
        </div>
        <div id="prompt-output" class="prompt-output" style="display:none">
          <div class="prompt-output-tabs">
            <button class="prompt-output-tab prompt-output-tab-active" data-output-tab="setup">Claude Code Setup</button>
            <button class="prompt-output-tab" data-output-tab="rules">rules.md</button>
            <button class="prompt-output-tab" data-output-tab="workflow">workflow.md</button>
          </div>
          <div id="prompt-out-setup" class="prompt-output-content prompt-tab-visible">
            <div class="prompt-cli-setup">
              <p>Installiere den Plankton Skill f&uuml;r Claude Code mit der CLI:</p>
              <pre class="prompt-content" id="prompt-out-setup-pre"></pre>
              <p class="prompt-token-hint">Die CLI f&uuml;hrt automatisch den Login durch und richtet die Secrets ein.</p>
            </div>
          </div>
          <div id="prompt-out-rules" class="prompt-output-content">
            <pre class="prompt-content" id="prompt-out-rules-pre"></pre>
          </div>
          <div id="prompt-out-workflow" class="prompt-output-content">
            <pre class="prompt-content" id="prompt-out-workflow-pre"></pre>
          </div>
          <div class="modal-actions">
            <button id="prompt-out-copy" class="btn-primary">In Zwischenablage kopieren</button>
            <button id="prompt-out-download" class="btn-small">&#8615; Herunterladen</button>
          </div>
        </div>
      </div>
    </div>
  </div>

  <!-- CLI-Modal (Install CLI) -->
  <div id="cli-modal" class="modal-overlay">
    <div class="modal modal-wide">
      <div class="modal-header">
        <span class="modal-heading">Plankton CLI</span>
        <button class="modal-close" id="cli-modal-close">&#10005;</button>
      </div>
      <div class="prompt-skill-info">
        <h3>Installation</h3>
        <p>Installiere die Plankton CLI mit einem Befehl:</p>
        <pre class="prompt-content" id="cli-install-cmd">curl -fsSL .../install | bash</pre>
        <div class="modal-actions">
          <button class="btn-small" data-cli-copy="cli-install-cmd">In Zwischenablage kopieren</button>
        </div>

        <h3>Login</h3>
        <p>Server hinzuf&uuml;gen und einloggen (wie <code>git remote add</code>):</p>
        <pre class="prompt-content" id="cli-login-cmd">plankton remote add origin ...</pre>
        <div class="modal-actions">
          <button class="btn-small" data-cli-copy="cli-login-cmd">In Zwischenablage kopieren</button>
        </div>

        <h3>Claude Code Skill</h3>
        <p>Skill installieren (inkl. Login + Secrets-Setup):</p>
        <pre class="prompt-content" id="cli-skill-cmd">plankton skill install ... --global</pre>
        <div class="modal-actions">
          <button class="btn-small" data-cli-copy="cli-skill-cmd">In Zwischenablage kopieren</button>
        </div>

        <h3>Hilfe</h3>
        <pre class="prompt-content">plankton help                    # Alle Befehle anzeigen
plankton remote add origin ...   # Login + Update: gleicher Befehl
curl -fsSL .../install | bash    # CLI aktualisieren</pre>
      </div>
    </div>
  </div>

  <!-- Admin-Modal (Legacy) -->
  <div id="admin-modal" class="modal-overlay">
    <div class="modal modal-wide">
      <div class="modal-header">
        <span class="modal-heading">Administration</span>
        <button class="modal-close" id="admin-modal-close">&#10005;</button>
      </div>
      <div class="admin-tabs">
        <button class="admin-tab admin-tab-active" data-tab="users">Nutzer</button>
        <button class="admin-tab" data-tab="tokens">Tokens</button>
      </div>
      <div id="admin-user-list" class="admin-user-list"></div>
      <div id="admin-user-form" class="admin-user-form" style="display:none">
        <label>Username <input id="admin-username" type="text" /></label>
        <label>Anzeigename <input id="admin-displayname" type="text" /></label>
        <label>Passwort <input id="admin-password" type="password" /></label>
        <label>Rolle
          <select id="admin-role">
            <option value="user">User</option>
            <option value="admin">Admin</option>
          </select>
        </label>
        <div class="modal-actions">
          <button id="admin-form-save" class="btn-primary">Speichern</button>
          <button id="admin-form-cancel" class="btn-small">Abbrechen</button>
        </div>
      </div>
      <div class="modal-actions" id="admin-list-actions">
        <button id="admin-add-user-btn" class="btn-primary">Neuer Nutzer</button>
      </div>
      <div id="admin-token-section" style="display:none">
        <div id="admin-token-list" class="admin-user-list"></div>
        <div class="admin-token-create">
          <input id="admin-token-name" type="text" placeholder="Token-Name..." />
          <select id="admin-token-role">
            <option value="developer">Developer</option>
            <option value="tester">Tester</option>
            <option value="manager">Manager</option>
          </select>
          <button id="admin-create-token-btn" class="btn-primary">Token erstellen</button>
        </div>
        <pre id="admin-token-result" class="token-result" style="display:none"></pre>
      </div>
    </div>
  </div>

  <!-- Passwort-Modal (Legacy) -->
  <div id="password-modal" class="modal-overlay">
    <div class="modal">
      <div class="modal-header">
        <span class="modal-heading">Passwort ändern</span>
        <button class="modal-close" id="pw-modal-close">&#10005;</button>
      </div>
      <div id="pw-error" class="login-error"></div>
      <label>Altes Passwort <input id="pw-old" type="password" /></label>
      <label>Neues Passwort <input id="pw-new" type="password" /></label>
      <label>Neues Passwort bestätigen <input id="pw-confirm" type="password" /></label>
      <div class="modal-actions">
        <button id="pw-save-btn" class="btn-primary">Speichern</button>
      </div>
    </div>
  </div>

  <!-- Import-Modal (Legacy) -->
  <div id="import-modal" class="modal-overlay">
    <div class="modal modal-wide">
      <div class="modal-header">
        <span class="modal-heading">Issues importieren</span>
        <button class="modal-close" id="import-modal-close">&#10005;</button>
      </div>
      <label>JSON (Array von Tasks)
        <textarea id="import-json" rows="10" placeholder='[{"title": "...", "column_slug": "TODO", "points": 3, "labels": ["feature"]}]' spellcheck="false"></textarea>
      </label>
      <div class="modal-actions">
        <button id="import-validate-btn" class="btn-small">Validieren</button>
        <button id="import-start-btn" class="btn-primary" style="display:none">Import starten</button>
      </div>
      <div id="import-preview" class="import-preview" style="display:none"></div>
      <div id="import-result" class="import-result" style="display:none"></div>
    </div>
  </div>
</template>
