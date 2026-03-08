// DOM aufbauen und Event-Listener registrieren.

import { state } from './state.js';
import { initTheme, toggleTheme } from './components/theme.js';
import { doLogout, updateUserSection } from './components/auth.js';
import { renderBoard } from './components/board.js';
import { openNewTaskModal, openTaskModal, closeTaskModal, renderModalComments } from './components/task-modal.js';
import { openTaskDetail, closeTaskDetail } from './components/task-detail.js';
import { updateBulkBar, bulkDeleteSelected } from './components/bulk-actions.js';
import { closeColumnModal, saveColumnModal, selectColor } from './components/column-modal.js';
import { openColumnMenu } from './components/column-modal.js';
import { openProjectDropdown, closeProjectMenu, copyProjectJson, importProjectJson, saveProjectJson, saveProjectTitle, closePromptModal } from './components/project-menu.js';
import { toggleJsonView } from './components/json-view.js';
import { openAdminModal, closeAdminModal, showAdminForm, saveAdminForm, switchAdminTab, createToken, handleTokenAction, handleAdminUserAction } from './components/admin.js';
import { openPasswordModal, closePasswordModal, savePassword } from './components/password-modal.js';
import { openImportModal, closeImportModal, validateImport, executeImport } from './components/import-modal.js';
import { openGitModal, closeGitModal, saveGitConfig, triggerGitSync } from './components/git-settings.js';
import { createProject, saveTask, createTaskViaApi, deleteTask, loadProjects, openProject } from './services/project-service.js';

export function buildDOM(showLoginPage) {
  document.body.innerHTML = `
    <div class="app">
      <aside class="sidebar">
        <div class="sidebar-header">
          <span class="logo">🪼 Plankton</span>
          <button id="theme-toggle" class="theme-toggle" title="Theme wechseln">☀</button>
        </div>
        <div class="sidebar-create">
          <input id="new-project-input" placeholder="Projektname…" />
          <button id="new-project-btn">Erstellen</button>
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

      <main class="main">
        <header class="board-header">
          <h1 id="project-title" class="board-title"></h1>
          <span id="git-status-icon" class="git-status-icon" style="display:none" title="Git"></span>
          <button id="import-btn" class="import-btn" title="Issues importieren">&#8615; Import</button>
          <button id="project-menu-btn" class="project-menu-btn" title="Projekt-Menü">&#9776;</button>
          <div id="project-dropdown" class="project-dropdown"></div>
        </header>
        <div id="bulk-bar" class="bulk-bar">
          <span><strong id="bulk-count">0</strong> Task(s) ausgewählt</span>
          <button id="bulk-delete-btn" class="btn-danger btn-small">Ausgewählte löschen</button>
          <button id="bulk-cancel-btn" class="btn-small">Auswahl aufheben</button>
        </div>
        <div id="board" class="board"></div>
      </main>
    </div>

    <div id="task-modal" class="modal-overlay">
      <div class="modal modal-wide">
        <div class="modal-header">
          <span class="modal-heading" id="modal-heading-text">Task bearbeiten</span>
          <button class="modal-close" id="modal-close-btn">&#10005;</button>
        </div>
        <div class="modal-grid">
          <div class="modal-col-main">
            <label>Titel
              <input id="modal-title" type="text" />
            </label>
            <label>Beschreibung
              <textarea id="modal-desc" rows="8"></textarea>
            </label>
            <label>Labels <small>(kommagetrennt)</small>
              <input id="modal-labels" type="text" />
            </label>
            <div class="modal-section">
              <span class="modal-section-title">Kommentare</span>
              <div id="modal-comments" class="modal-list"></div>
              <div class="comment-input-row">
                <input id="modal-new-comment" type="text" placeholder="Kommentar schreiben…" />
                <button id="modal-add-comment-btn" class="btn-small">+</button>
              </div>
            </div>
          </div>
          <div class="modal-col-side">
            <label>Points <small>(0–100)</small>
              <input id="modal-points" type="number" min="0" max="100" />
            </label>
            <label>Worker
              <input id="modal-worker" type="text" />
            </label>
            <div class="modal-info">
              <span class="modal-info-label">Erstellt</span>
              <span id="modal-created" class="modal-info-value">–</span>
            </div>
            <div class="modal-info">
              <span class="modal-info-label">Geändert</span>
              <span id="modal-updated" class="modal-info-value">–</span>
            </div>
            <div class="modal-info">
              <span class="modal-info-label">Vorherige Spalte</span>
              <span id="modal-prev-row" class="modal-info-value">–</span>
            </div>
            <div class="modal-section">
              <span class="modal-section-title">Logs</span>
              <div id="modal-logs" class="modal-list modal-list-small"></div>
            </div>
          </div>
        </div>
        <div class="modal-actions">
          <button id="modal-save-btn" class="btn-primary">Speichern</button>
          <button id="modal-delete-btn" class="btn-danger">Löschen</button>
        </div>
      </div>
    </div>

    <div id="task-detail-modal" class="modal-overlay">
      <div class="modal modal-detail">
        <div class="modal-header">
          <span class="modal-heading" id="detail-heading">Task</span>
          <button class="modal-close" id="detail-close-btn">&#10005;</button>
        </div>
        <div id="detail-title" class="detail-title"></div>
        <div id="detail-column-info"></div>
        <div class="detail-grid">
          <div class="detail-col-main">
            <div class="detail-section">
              <span class="detail-section-title">Beschreibung</span>
              <div id="detail-description" class="detail-description"></div>
            </div>
            <div class="detail-section">
              <span class="detail-section-title">Labels</span>
              <div id="detail-labels" class="detail-labels"></div>
            </div>
            <div class="detail-section">
              <span class="detail-section-title">Kommentare</span>
              <div id="detail-comments" class="detail-list"></div>
            </div>
          </div>
          <div class="detail-col-side">
            <div class="detail-section">
              <span class="detail-section-title">Details</span>
              <div class="detail-info-grid">
                <div class="detail-info-item">
                  <span class="detail-info-item-label">Points</span>
                  <span id="detail-points" class="detail-info-item-value">–</span>
                </div>
                <div class="detail-info-item">
                  <span class="detail-info-item-label">Worker</span>
                  <span id="detail-worker" class="detail-info-item-value">–</span>
                </div>
                <div class="detail-info-item">
                  <span class="detail-info-item-label">Erstellt</span>
                  <span id="detail-created" class="detail-info-item-value">–</span>
                </div>
                <div class="detail-info-item">
                  <span class="detail-info-item-label">Geändert</span>
                  <span id="detail-updated" class="detail-info-item-value">–</span>
                </div>
              </div>
            </div>
            <div class="detail-section">
              <span class="detail-section-title">Vorherige Spalte</span>
              <div id="detail-prev-row"></div>
            </div>
            <div class="detail-section">
              <span class="detail-section-title">Logs</span>
              <div id="detail-logs" class="detail-list"></div>
            </div>
          </div>
        </div>
        <div class="modal-actions">
          <button id="detail-edit-btn" class="btn-primary">Bearbeiten</button>
        </div>
      </div>
    </div>

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

    <div id="project-modal" class="modal-overlay">
      <div class="modal modal-wide">
        <div class="modal-header">
          <span class="modal-heading">Projekt</span>
          <button class="modal-close" id="proj-modal-close">&#10005;</button>
        </div>
        <label>Projektname
          <input id="proj-modal-title" type="text" placeholder="Projektname…" />
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

    <div id="prompt-modal" class="modal-overlay">
      <div class="modal modal-wide">
        <div class="modal-header">
          <span class="modal-heading">KI-Prompt</span>
          <button class="modal-close" id="prompt-modal-close">&#10005;</button>
        </div>
        <pre id="prompt-content" class="prompt-content"></pre>
        <div class="modal-actions">
          <button id="prompt-copy-btn" class="btn-primary">In Zwischenablage kopieren</button>
        </div>
      </div>
    </div>

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

    <div id="password-modal" class="modal-overlay">
      <div class="modal">
        <div class="modal-header">
          <span class="modal-heading">Passwort &auml;ndern</span>
          <button class="modal-close" id="pw-modal-close">&#10005;</button>
        </div>
        <div id="pw-error" class="login-error"></div>
        <label>Altes Passwort <input id="pw-old" type="password" /></label>
        <label>Neues Passwort <input id="pw-new" type="password" /></label>
        <label>Neues Passwort best&auml;tigen <input id="pw-confirm" type="password" /></label>
        <div class="modal-actions">
          <button id="pw-save-btn" class="btn-primary">Speichern</button>
        </div>
      </div>
    </div>

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
  `;

  // Projekt erstellen.
  document.getElementById('new-project-btn').addEventListener('click', () => {
    const input = document.getElementById('new-project-input');
    createProject(input.value.trim());
    input.value = '';
  });
  document.getElementById('new-project-input').addEventListener('keydown', e => {
    if (e.key === 'Enter') document.getElementById('new-project-btn').click();
  });

  // Board: Delegierter Click-Handler.
  document.getElementById('board').addEventListener('click', e => {
    if (state.isDragging) return;

    const checkbox = e.target.closest('.task-checkbox');
    if (checkbox) {
      e.stopPropagation();
      const taskId = checkbox.dataset.taskId;
      if (checkbox.checked) {
        state.selectedTasks.add(taskId);
      } else {
        state.selectedTasks.delete(taskId);
      }
      const inner = checkbox.closest('.task-inner');
      if (inner) inner.classList.toggle('task-selected', checkbox.checked);
      updateBulkBar();
      return;
    }

    const inner = e.target.closest('[data-task-id]');
    if (inner && !e.target.closest('.task-checkbox')) {
      const task = state.project.tasks.find(t => t.id === inner.dataset.taskId);
      if (task) openTaskDetail(task);
      return;
    }

    const addBtn = e.target.closest('.col-add-btn');
    if (addBtn) {
      openNewTaskModal(addBtn.dataset.colId);
      return;
    }

    const menuBtn = e.target.closest('.col-menu-btn');
    if (menuBtn) {
      e.stopPropagation();
      openColumnMenu(menuBtn, menuBtn.dataset.colId);
      return;
    }
  });

  // Detail-Ansicht.
  document.getElementById('detail-close-btn').addEventListener('click', closeTaskDetail);
  document.getElementById('task-detail-modal').addEventListener('click', e => {
    if (e.target.id === 'task-detail-modal') closeTaskDetail();
  });
  document.getElementById('detail-edit-btn').addEventListener('click', () => {
    if (!state.detailTask) return;
    const task = state.detailTask;
    closeTaskDetail();
    openTaskModal(task, false);
  });

  // Task-Modal.
  document.getElementById('modal-close-btn').addEventListener('click', closeTaskModal);
  document.getElementById('task-modal').addEventListener('click', e => {
    if (e.target.id === 'task-modal') closeTaskModal();
  });

  document.getElementById('modal-save-btn').addEventListener('click', async () => {
    if (!state.editingTask) return;
    const task = {
      ...state.editingTask,
      title: document.getElementById('modal-title').value || 'Untitled',
      description: document.getElementById('modal-desc').value,
      labels: document.getElementById('modal-labels').value
          .split(',').map(s => s.trim()).filter(Boolean),
      points: parseInt(document.getElementById('modal-points').value, 10) || 0,
      worker: document.getElementById('modal-worker').value.trim(),
    };

    if (state.isNewTask) {
      await createTaskViaApi(task);
    } else {
      await saveTask(task);
    }
    closeTaskModal();
  });

  document.getElementById('modal-delete-btn').addEventListener('click', () => {
    if (!state.editingTask) return;
    if (confirm(`Task "${state.editingTask.title}" wirklich löschen?`)) {
      deleteTask(state.editingTask.id);
      closeTaskModal();
    }
  });

  // Kommentar hinzufügen.
  document.getElementById('modal-add-comment-btn').addEventListener('click', () => {
    const input = document.getElementById('modal-new-comment');
    const text = input.value.trim();
    if (!text || !state.editingTask) return;
    state.editingTask.comments.push(text);
    input.value = '';
    renderModalComments();
  });
  document.getElementById('modal-new-comment').addEventListener('keydown', e => {
    if (e.key === 'Enter') document.getElementById('modal-add-comment-btn').click();
  });

  // Bulk-Aktionen.
  document.getElementById('bulk-delete-btn').addEventListener('click', bulkDeleteSelected);
  document.getElementById('bulk-cancel-btn').addEventListener('click', () => {
    state.selectedTasks.clear();
    renderBoard();
  });

  // Spalten-Modal.
  document.getElementById('col-modal-close').addEventListener('click', closeColumnModal);
  document.getElementById('column-modal').addEventListener('click', e => {
    if (e.target.id === 'column-modal') closeColumnModal();
  });
  document.getElementById('col-modal-save').addEventListener('click', saveColumnModal);
  document.getElementById('col-modal-title').addEventListener('keydown', e => {
    if (e.key === 'Enter') saveColumnModal();
    if (e.key === 'Escape') closeColumnModal();
  });
  document.getElementById('col-modal-colors').addEventListener('click', e => {
    const swatch = e.target.closest('.color-swatch');
    if (!swatch) return;
    selectColor(swatch.dataset.color);
  });

  // Projekt-Menü.
  document.getElementById('project-menu-btn').addEventListener('click', (e) => {
    e.stopPropagation();
    openProjectDropdown();
  });

  // Projekt-Editieren Modal.
  document.getElementById('proj-modal-close').addEventListener('click', closeProjectMenu);
  document.getElementById('project-modal').addEventListener('click', e => {
    if (e.target.id === 'project-modal') closeProjectMenu();
  });
  document.getElementById('proj-modal-copy').addEventListener('click', copyProjectJson);
  document.getElementById('proj-modal-import').addEventListener('click', importProjectJson);
  document.getElementById('proj-modal-save').addEventListener('click', saveProjectJson);
  document.getElementById('proj-modal-title').addEventListener('keydown', e => {
    if (e.key === 'Enter') saveProjectTitle();
  });
  document.getElementById('proj-view-toggle').addEventListener('click', toggleJsonView);

  // Prompt-Modal.
  document.getElementById('prompt-modal-close').addEventListener('click', closePromptModal);
  document.getElementById('prompt-modal').addEventListener('click', e => {
    if (e.target.id === 'prompt-modal') closePromptModal();
  });
  document.getElementById('prompt-copy-btn').addEventListener('click', async () => {
    const text = document.getElementById('prompt-content').textContent;
    try {
      await navigator.clipboard.writeText(text);
      const btn = document.getElementById('prompt-copy-btn');
      btn.textContent = 'Kopiert!';
      setTimeout(() => { btn.textContent = 'In Zwischenablage kopieren'; }, 1500);
    } catch {}
  });

  // User-Aktionen.
  document.getElementById('logout-btn').addEventListener('click', () => doLogout(showLoginPage));
  document.getElementById('password-btn').addEventListener('click', () => openPasswordModal(false));
  document.getElementById('admin-btn').addEventListener('click', openAdminModal);

  // Admin-Modal.
  document.getElementById('admin-modal-close').addEventListener('click', closeAdminModal);
  document.getElementById('admin-modal').addEventListener('click', e => {
    if (e.target.id === 'admin-modal') closeAdminModal();
  });
  document.getElementById('admin-add-user-btn').addEventListener('click', () => showAdminForm(null));
  document.getElementById('admin-form-save').addEventListener('click', saveAdminForm);
  document.getElementById('admin-form-cancel').addEventListener('click', () => openAdminModal());
  document.querySelectorAll('.admin-tab').forEach(tab => {
    tab.addEventListener('click', () => switchAdminTab(tab.dataset.tab));
  });
  document.getElementById('admin-create-token-btn').addEventListener('click', createToken);
  document.getElementById('admin-token-list').addEventListener('click', async (e) => {
    const btn = e.target.closest('[data-token-action]');
    if (!btn) return;
    handleTokenAction(btn.dataset.tokenAction, btn.dataset.tid);
  });
  document.getElementById('admin-user-list').addEventListener('click', async (e) => {
    const btn = e.target.closest('[data-admin-action]');
    if (!btn) return;
    handleAdminUserAction(btn.dataset.adminAction, btn.dataset.uid);
  });

  // Passwort-Modal.
  document.getElementById('pw-modal-close').addEventListener('click', closePasswordModal);
  document.getElementById('password-modal').addEventListener('click', e => {
    if (e.target.id === 'password-modal') closePasswordModal();
  });
  document.getElementById('pw-save-btn').addEventListener('click', savePassword);

  // Import-Modal.
  document.getElementById('import-btn').addEventListener('click', openImportModal);
  document.getElementById('import-modal-close').addEventListener('click', closeImportModal);
  document.getElementById('import-modal').addEventListener('click', e => {
    if (e.target.id === 'import-modal') closeImportModal();
  });
  document.getElementById('import-validate-btn').addEventListener('click', validateImport);
  document.getElementById('import-start-btn').addEventListener('click', executeImport);

  // Git-Status-Icon im Header.
  document.getElementById('git-status-icon').addEventListener('click', () => openGitModal());

  // Git-Modal.
  document.getElementById('git-modal-close').addEventListener('click', closeGitModal);
  document.getElementById('git-modal').addEventListener('click', e => {
    if (e.target.id === 'git-modal') closeGitModal();
  });
  document.getElementById('git-save-btn').addEventListener('click', saveGitConfig);
  document.getElementById('git-sync-btn').addEventListener('click', triggerGitSync);
}
