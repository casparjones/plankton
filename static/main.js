import './styles.css';
// jKanban wird via imports-loader (this→window) + exports-loader (window.jKanban→default)
// in webpack.config.js als CommonJS-Export bereitgestellt.
import jKanban from 'jkanban';

// ============================================================
// Plankton Frontend – jKanban + Vanilla JS
// ============================================================

// ------------------------------------------------------------------
// API-Client
// ------------------------------------------------------------------

const api = {
  async get(path) {
    const r = await fetch(path);
    if (!r.ok) throw new Error(`GET ${path} → ${r.status}`);
    return r.json();
  },
  async post(path, body) {
    const r = await fetch(path, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    if (!r.ok) throw new Error(`POST ${path} → ${r.status}`);
    return r.json();
  },
  async put(path, body) {
    const r = await fetch(path, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    if (!r.ok) throw new Error(`PUT ${path} → ${r.status}`);
    return r.json();
  },
  async del(path) {
    const r = await fetch(path, { method: 'DELETE' });
    if (!r.ok) throw new Error(`DELETE ${path} → ${r.status}`);
  },
};

// ------------------------------------------------------------------
// State
// ------------------------------------------------------------------

const state = {
  projects: [],
  project: null,
  kanban: null,
  editingTask: null,    // Task im Modal (null = kein Modal offen)
  isNewTask: false,     // true = Modal ist für neuen (noch nicht gespeicherten) Task
  selectedTasks: new Set(), // IDs der selektierten Tasks für Bulk-Aktionen
  eventSource: null,
  currentUser: null,
  isDragging: false,    // true während ein Task/Board gezogen wird
  detailTask: null,     // Task in der Detail-Ansicht
};

// 20 vordefinierte Farben für Spalten.
const COLUMN_COLORS = [
  '#90CAF9', '#FFCC80', '#A5D6A7', '#EF9A9A', '#CE93D8',
  '#80DEEA', '#FFF59D', '#FFAB91', '#B0BEC5', '#F48FB1',
  '#81D4FA', '#C5E1A5', '#BCAAA4', '#B39DDB', '#80CBC4',
  '#FFE082', '#9FA8DA', '#E6EE9C', '#FFCCBC', '#D1C4E9',
];

// ------------------------------------------------------------------
// Theme (Dark/Light Mode)
// ------------------------------------------------------------------

function applyTheme(theme) {
  document.body.setAttribute('data-theme', theme);
  localStorage.setItem('plankton-theme', theme);
  const toggle = document.getElementById('theme-toggle');
  if (toggle) toggle.textContent = theme === 'dark' ? '\u2600' : '\u263E';
}

function toggleTheme() {
  const current = document.body.getAttribute('data-theme') || 'dark';
  applyTheme(current === 'dark' ? 'light' : 'dark');
}

function initTheme() {
  const stored = localStorage.getItem('plankton-theme');
  if (stored) {
    applyTheme(stored);
  } else {
    const prefersLight = window.matchMedia('(prefers-color-scheme: light)').matches;
    applyTheme(prefersLight ? 'light' : 'dark');
  }
}

// ------------------------------------------------------------------
// Auth-Funktionen
// ------------------------------------------------------------------

async function checkAuth() {
  try {
    const r = await fetch('/auth/me');
    if (!r.ok) return null;
    return await r.json();
  } catch {
    return null;
  }
}

async function doLogin(username, password) {
  const r = await fetch('/auth/login', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ username, password }),
  });
  if (!r.ok) {
    const err = await r.json().catch(() => ({ error: 'Login fehlgeschlagen' }));
    throw new Error(err.error || 'Login fehlgeschlagen');
  }
  return await r.json();
}

async function doLogout() {
  await fetch('/auth/logout', { method: 'POST' });
  state.currentUser = null;
  showLoginPage();
}

async function doChangePassword(oldPassword, newPassword) {
  const r = await fetch('/auth/change-password', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ old_password: oldPassword, new_password: newPassword }),
  });
  if (!r.ok) {
    const err = await r.json().catch(() => ({ error: 'Fehler' }));
    throw new Error(err.error || 'Passwort-Änderung fehlgeschlagen');
  }
  return await r.json();
}

function showLoginPage() {
  document.body.innerHTML = `
    <div class="login-page">
      <div class="login-card">
        <div class="login-logo"><img src="/icons/favicon-64.png" alt="Plankton" class="login-logo-img" /> Plankton</div>
        <div id="login-error" class="login-error"></div>
        <form id="login-form">
          <label>Username
            <input id="login-username" type="text" autocomplete="username" autofocus />
          </label>
          <label>Passwort
            <input id="login-password" type="password" autocomplete="current-password" />
          </label>
          <button type="submit" class="btn-primary login-btn">Anmelden</button>
        </form>
      </div>
    </div>
  `;
  document.getElementById('login-form').addEventListener('submit', async (e) => {
    e.preventDefault();
    const username = document.getElementById('login-username').value.trim();
    const password = document.getElementById('login-password').value;
    const errEl = document.getElementById('login-error');
    errEl.textContent = '';
    try {
      await doLogin(username, password);
      const user = await checkAuth();
      if (user) {
        state.currentUser = user;
        if (user.must_change_password) {
          await startApp();
          setTimeout(() => openPasswordModal(true), 100);
        } else {
          await startApp();
        }
      }
    } catch (err) {
      errEl.textContent = err.message;
    }
  });
}

function updateUserSection() {
  const user = state.currentUser;
  if (!user) return;
  const avatarEl = document.getElementById('user-avatar');
  const nameEl = document.getElementById('user-name');
  const roleEl = document.getElementById('user-role');
  const adminBtn = document.getElementById('admin-btn');

  if (avatarEl) avatarEl.textContent = (user.display_name || user.username || '?')[0].toUpperCase();
  if (nameEl) nameEl.textContent = user.display_name || user.username;
  if (roleEl) roleEl.textContent = user.role;
  if (adminBtn) adminBtn.style.display = user.role === 'admin' ? '' : 'none';
}

async function startApp() {
  buildDOM();
  initTheme();
  document.getElementById('theme-toggle').addEventListener('click', toggleTheme);
  updateUserSection();
  await loadProjects();
  if (state.projects.length > 0) {
    await openProject(state.projects[0]._id);
  }
}

// ------------------------------------------------------------------
// Hilfsfunktionen
// ------------------------------------------------------------------

function tasksForColumn(columnId) {
  return (state.project?.tasks || [])
      .filter(t => t.column_id === columnId)
      .sort((a, b) => a.order - b.order);
}

function taskToItem(task) {
  const isSelected = state.selectedTasks.has(task.id);
  const labels = (task.labels || [])
      .map(l => `<span class="label">${escapeHtml(l)}</span>`)
      .join('');
  const pointsBadge = task.points
      ? `<span class="points-badge">${task.points}</span>`
      : '';
  const workerAvatar = task.worker
      ? `<span class="avatar" title="${escapeHtml(task.worker)}">${escapeHtml(task.worker[0].toUpperCase())}</span>`
      : '';

  return {
    id: task.id,
    title: `
      <div class="task-inner ${isSelected ? 'task-selected' : ''}" data-task-id="${task.id}">
        <div class="task-header-row">
          <input type="checkbox" class="task-checkbox" data-task-id="${task.id}" ${isSelected ? 'checked' : ''} />
          <div class="task-title">${escapeHtml(task.title)}</div>
          ${pointsBadge}
        </div>
        ${task.description
        ? `<div class="task-desc">${escapeHtml(task.description.substring(0, 80))}${task.description.length > 80 ? '…' : ''}</div>`
        : ''}
        <div class="task-meta">
          <div class="task-labels">${labels}</div>
          <div class="task-assignees">${workerAvatar}</div>
        </div>
      </div>
    `,
  };
}

function escapeHtml(str) {
  return String(str)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;');
}

function columnName(colId) {
  if (!colId) return '–';
  const col = (state.project?.columns || []).find(c => c.id === colId);
  return col ? col.title : '–';
}

function formatDate(isoStr) {
  if (!isoStr) return '–';
  try {
    return new Date(isoStr).toLocaleString('de-DE', {
      year: 'numeric', month: '2-digit', day: '2-digit',
      hour: '2-digit', minute: '2-digit',
    });
  } catch { return isoStr; }
}

// ------------------------------------------------------------------
// Board rendern
// ------------------------------------------------------------------

function renderBoard() {
  if (!state.project) return;

  if (state.kanban) {
    try { state.kanban.destroy(); } catch (_) {}
    state.kanban = null;
  }

  const container = document.getElementById('board');
  container.innerHTML = '';

  // Bulk-Aktions-Leiste aktualisieren.
  updateBulkBar();

  const columns = [...(state.project.columns || [])]
      .filter(c => !c.hidden)
      .sort((a, b) => {
        // Done-Spalte immer als letztes
        const aIsDone = a.title === 'Done';
        const bIsDone = b.title === 'Done';
        if (aIsDone && !bIsDone) return 1;
        if (!aIsDone && bIsDone) return -1;
        return a.order - b.order;
      });

  const boards = columns.map(col => {
    const tasks = tasksForColumn(col.id);
    const count = tasks.length;
    const isDone = col.title === 'Done';
    return {
      id: col.id,
      title: `
        <span class="col-title" style="border-color:${col.color}">${escapeHtml(col.title)}</span>
        <span class="col-count">${count}</span>
        <div class="col-actions">
          <button class="col-add-btn" data-col-id="${col.id}" title="Task hinzufügen">+</button>
          <button class="col-menu-btn" data-col-id="${col.id}" title="Spalte verwalten">&#9776;</button>
        </div>
      `,
      item: tasks.map(taskToItem),
      class: isDone ? 'kanban-column,col-done' : 'kanban-column',
    };
  });

  state.kanban = new jKanban({
    element: '#board',
    gutter: '0',
    widthBoard: '300px',
    responsivePercentage: false,
    dragBoards: true,
    addItemButton: false,
    boards,

    dragEl(el) {
      el.classList.add('dragging');
      state.isDragging = true;
    },
    dragendEl(el) {
      el.classList.remove('dragging');
      // Kurze Verzögerung, damit der Click-Handler nach dem Drop nicht feuert.
      setTimeout(() => { state.isDragging = false; }, 50);
    },
    dropEl(el, target) {
      const taskId = el.querySelector('[data-task-id]')?.dataset.taskId;
      const columnId = target.closest('.kanban-board')?.dataset.id;
      if (!taskId || !columnId) return;
      const items = [...target.querySelectorAll('[data-task-id]')];
      const newOrder = items.findIndex(i => i.dataset.taskId === taskId);
      moveTask(taskId, columnId, Math.max(0, newOrder), true);
    },
    dragBoard(el) {
      state.isDragging = true;
      // Done-Spalte darf nicht verschoben werden
      const colId = el.dataset.id;
      const col = state.project.columns.find(c => c.id === colId);
      if (col && col.title === 'Done') {
        state.kanban.drakeBoard.cancel(true);
      }
    },
    dragendBoard(el) {
      setTimeout(() => { state.isDragging = false; }, 50);
    },
    dropBoard(el, target, source, sibling) {
      reorderColumnsFromDOM();
    },
  });
}

// ------------------------------------------------------------------
// Bulk-Aktions-Leiste
// ------------------------------------------------------------------

function updateBulkBar() {
  const bar = document.getElementById('bulk-bar');
  if (!bar) return;
  const count = state.selectedTasks.size;
  if (count === 0) {
    bar.classList.remove('visible');
  } else {
    bar.classList.add('visible');
    document.getElementById('bulk-count').textContent = count;
  }
}

async function bulkDeleteSelected() {
  const ids = [...state.selectedTasks];
  if (ids.length === 0) return;
  if (!confirm(`${ids.length} Task(s) wirklich löschen?`)) return;

  // Alle Tasks sequentiell löschen.
  for (const taskId of ids) {
    try {
      await api.del(`/api/projects/${state.project._id}/tasks/${taskId}`);
    } catch (err) {
      console.error('Fehler beim Löschen:', taskId, err);
    }
  }
  state.selectedTasks.clear();
  state.project = await api.get(`/api/projects/${state.project._id}`);
  renderBoard();
}

// ------------------------------------------------------------------
// API-Aktionen
// ------------------------------------------------------------------

async function loadProjects() {
  state.projects = await api.get('/api/projects');
  renderProjectList();
}

async function openProject(id) {
  state.project = await api.get(`/api/projects/${id}`);
  state.selectedTasks.clear();
  renderProjectList();
  renderBoard();
  updateProjectTitle();
  subscribeSSE(id);
}

async function createProject(title) {
  const payload = {
    _id: '',
    title: title || 'Untitled',
    columns: [
      { id: crypto.randomUUID(), title: 'Todo',        order: 0, color: '#90CAF9', hidden: false },
      { id: crypto.randomUUID(), title: 'In Progress', order: 1, color: '#FFCC80', hidden: false },
      { id: crypto.randomUUID(), title: 'Done',        order: 2, color: '#A5D6A7', hidden: false },
      { id: crypto.randomUUID(), title: '_archive',    order: 99, color: '#444',   hidden: true },
    ],
    users: [],
    tasks: [],
  };
  state.project = await api.post('/api/projects', payload);
  await loadProjects();
  renderBoard();
  updateProjectTitle();
  subscribeSSE(state.project._id);
}

async function renameProject(id, newTitle) {
  const project = await api.get(`/api/projects/${id}`);
  project.title = newTitle;
  await api.put(`/api/projects/${id}`, project);
  await loadProjects();
  if (state.project?._id === id) {
    state.project.title = newTitle;
    updateProjectTitle();
  }
}

async function deleteProject(id) {
  const project = await api.get(`/api/projects/${id}?include_archived=true`);
  const rev = project._rev;
  await api.del(`/api/projects/${id}?rev=${rev}`);
  await loadProjects();
  if (state.project?._id === id) {
    if (state.projects.length > 0) {
      await openProject(state.projects[0]._id);
    } else {
      state.project = null;
      renderBoard();
      updateProjectTitle();
    }
  }
}

async function saveTask(task) {
  state.project = await api.put(
      `/api/projects/${state.project._id}/tasks/${task.id}`,
      task
  );
  renderBoard();
}

async function createTaskViaApi(task) {
  state.project = await api.post(`/api/projects/${state.project._id}/tasks`, task);
  renderBoard();
}

async function deleteTask(taskId) {
  await api.del(`/api/projects/${state.project._id}/tasks/${taskId}`);
  state.project = await api.get(`/api/projects/${state.project._id}`);
  renderBoard();
}

async function moveTask(taskId, columnId, order, skipRender) {
  try {
    state.project = await api.post(
        `/api/projects/${state.project._id}/tasks/${taskId}/move`,
        { column_id: columnId, order }
    );
    if (!skipRender) renderBoard();
  } catch (err) {
    console.error('Move failed:', err);
    // Bei Fehler Board neu rendern, damit der DOM-Zustand konsistent ist.
    renderBoard();
  }
}

// ------------------------------------------------------------------
// Spalten-Management
// ------------------------------------------------------------------

async function addColumn(title, color) {
  const doneCol = state.project.columns.find(c => c.title === 'Done');
  let newOrder;
  if (doneCol) {
    newOrder = doneCol.order;
    doneCol.order = newOrder + 1;
    await api.put(`/api/projects/${state.project._id}/columns/${doneCol.id}`, doneCol);
  } else {
    newOrder = state.project.columns.length;
  }
  const col = { id: '', title, order: newOrder, color };
  state.project = await api.post(`/api/projects/${state.project._id}/columns`, col);
  renderBoard();
}

async function updateColumn(colId, title, color) {
  const col = state.project.columns.find(c => c.id === colId);
  if (!col) return;
  const updated = { ...col, title, color };
  state.project = await api.put(`/api/projects/${state.project._id}/columns/${colId}`, updated);
  renderBoard();
}

async function deleteColumn(colId) {
  const col = state.project.columns.find(c => c.id === colId);
  const taskCount = tasksForColumn(colId).length;
  const msg = taskCount > 0
      ? `Spalte "${col.title}" und ${taskCount} Task(s) wirklich löschen?`
      : `Spalte "${col.title}" wirklich löschen?`;
  if (!confirm(msg)) return;
  state.project = await api.del(`/api/projects/${state.project._id}/columns/${colId}`)
      .then(() => api.get(`/api/projects/${state.project._id}`));
  renderBoard();
}

// Spalten-Kontextmenü (Dropdown).
function openColumnMenu(anchorEl, colId) {
  closeColumnMenu();

  const col = state.project.columns.find(c => c.id === colId);
  if (!col) return;

  const visible = state.project.columns
    .filter(c => !c.hidden)
    .sort((a, b) => a.order - b.order);
  const idx = visible.findIndex(c => c.id === colId);
  const canMoveLeft = idx > 0 && col.title !== 'Done';
  const canMoveRight = idx < visible.length - 1 && col.title !== 'Done'
    && visible[idx + 1]?.title !== 'Done';

  const menu = document.createElement('div');
  menu.className = 'col-context-menu';
  menu.id = 'col-context-menu';

  menu.innerHTML = `
    <button class="col-ctx-item" data-action="edit">&#9998; Spalte bearbeiten</button>
    <button class="col-ctx-item" data-action="add">&#43; Neue Spalte</button>
    <div class="col-ctx-separator"></div>
    <button class="col-ctx-item${canMoveLeft ? '' : ' col-ctx-disabled'}" data-action="move-left" ${canMoveLeft ? '' : 'disabled'}>&#9664; Nach links</button>
    <button class="col-ctx-item${canMoveRight ? '' : ' col-ctx-disabled'}" data-action="move-right" ${canMoveRight ? '' : 'disabled'}>&#9654; Nach rechts</button>
    <div class="col-ctx-separator"></div>
    <button class="col-ctx-item col-ctx-danger" data-action="delete">&#10005; Spalte löschen</button>
  `;

  menu.addEventListener('click', (e) => {
    const action = e.target.closest('[data-action]')?.dataset.action;
    closeColumnMenu();
    if (action === 'edit') openColumnEditModal(colId);
    if (action === 'add') openColumnAddModal();
    if (action === 'delete') deleteColumn(colId);
    if (action === 'move-left') moveColumn(colId, 'left');
    if (action === 'move-right') moveColumn(colId, 'right');
  });

  const rect = anchorEl.getBoundingClientRect();
  menu.style.top = (rect.bottom + 4) + 'px';
  menu.style.left = rect.left + 'px';
  document.body.appendChild(menu);

  setTimeout(() => {
    document.addEventListener('click', closeColumnMenu, { once: true });
  }, 0);
}

function closeColumnMenu() {
  const existing = document.getElementById('col-context-menu');
  if (existing) existing.remove();
}

// ------------------------------------------------------------------
// Spalten-Modal (Overlay mit Farbauswahl)
// ------------------------------------------------------------------

let columnModalState = { mode: null, colId: null, selectedColor: null };

function openColumnEditModal(colId) {
  const col = state.project.columns.find(c => c.id === colId);
  if (!col) return;
  columnModalState = { mode: 'edit', colId, selectedColor: col.color };
  document.getElementById('col-modal-heading').textContent = 'Spalte bearbeiten';
  document.getElementById('col-modal-title').value = col.title;
  renderColorPicker(col.color);
  document.getElementById('column-modal').classList.add('open');
  setTimeout(() => document.getElementById('col-modal-title').focus(), 50);
}

function openColumnAddModal() {
  const usedColors = state.project.columns.map(c => c.color.toUpperCase());
  const suggested = COLUMN_COLORS.find(c => !usedColors.includes(c.toUpperCase())) || COLUMN_COLORS[0];
  columnModalState = { mode: 'add', colId: null, selectedColor: suggested };
  document.getElementById('col-modal-heading').textContent = 'Neue Spalte';
  document.getElementById('col-modal-title').value = '';
  renderColorPicker(suggested);
  document.getElementById('column-modal').classList.add('open');
  setTimeout(() => document.getElementById('col-modal-title').focus(), 50);
}

function renderColorPicker(selected) {
  const grid = document.getElementById('col-modal-colors');
  grid.innerHTML = COLUMN_COLORS.map(c => {
    const sel = c.toUpperCase() === (selected || '').toUpperCase() ? ' color-swatch-selected' : '';
    return `<button class="color-swatch${sel}" data-color="${c}" style="background:${c}" title="${c}"></button>`;
  }).join('');
}

function closeColumnModal() {
  document.getElementById('column-modal').classList.remove('open');
  columnModalState = { mode: null, colId: null, selectedColor: null };
}

async function saveColumnModal() {
  const title = document.getElementById('col-modal-title').value.trim();
  if (!title) return;
  const color = columnModalState.selectedColor || COLUMN_COLORS[0];
  if (columnModalState.mode === 'edit' && columnModalState.colId) {
    await updateColumn(columnModalState.colId, title, color);
  } else {
    await addColumn(title, color);
  }
  closeColumnModal();
}

// ------------------------------------------------------------------
// Spalten verschieben (Reihenfolge ändern)
// ------------------------------------------------------------------

async function reorderColumnsFromDOM() {
  const boardEls = [...document.querySelectorAll('#board .kanban-board')];
  const idOrder = boardEls.map(b => b.dataset.id);

  // Done-Spalte immer als letztes erzwingen
  const doneCol = state.project.columns.find(c => c.title === 'Done' && !c.hidden);
  if (doneCol) {
    const doneIdx = idOrder.indexOf(doneCol.id);
    if (doneIdx !== -1 && doneIdx !== idOrder.length - 1) {
      idOrder.splice(doneIdx, 1);
      idOrder.push(doneCol.id);
      // DOM korrigieren: Done-Board ans Ende verschieben
      renderBoard();
      return;
    }
  }

  // Order-Werte aktualisieren
  const updates = [];
  for (let i = 0; i < idOrder.length; i++) {
    const col = state.project.columns.find(c => c.id === idOrder[i]);
    if (col && col.order !== i) {
      col.order = i;
      updates.push(api.put(`/api/projects/${state.project._id}/columns/${col.id}`, col));
    }
  }

  if (updates.length > 0) {
    await Promise.all(updates);
    state.project = await api.get(`/api/projects/${state.project._id}`);
  }
}

async function moveColumn(colId, direction) {
  const visible = state.project.columns
    .filter(c => !c.hidden)
    .sort((a, b) => {
      const aIsDone = a.title === 'Done';
      const bIsDone = b.title === 'Done';
      if (aIsDone && !bIsDone) return 1;
      if (!aIsDone && bIsDone) return -1;
      return a.order - b.order;
    });
  const idx = visible.findIndex(c => c.id === colId);
  if (idx < 0) return;

  const targetIdx = direction === 'left' ? idx - 1 : idx + 1;
  if (targetIdx < 0 || targetIdx >= visible.length) return;

  const col = visible[idx];
  const target = visible[targetIdx];
  if (col.title === 'Done') return;
  if (target.title === 'Done') return;

  const tempOrder = col.order;
  col.order = target.order;
  target.order = tempOrder;

  await api.put(`/api/projects/${state.project._id}/columns/${col.id}`, col);
  await api.put(`/api/projects/${state.project._id}/columns/${target.id}`, target);
  state.project = await api.get(`/api/projects/${state.project._id}`);
  renderBoard();
}

// ------------------------------------------------------------------
// Projekt-Menü (Dropdown + Modals)
// ------------------------------------------------------------------

function openProjectDropdown() {
  closeProjectDropdown();
  if (!state.project) return;

  const btn = document.getElementById('project-menu-btn');
  const dropdown = document.getElementById('project-dropdown');
  dropdown.innerHTML = `
    <button class="proj-dropdown-item" data-action="edit">&#9998; Projekt editieren</button>
    <button class="proj-dropdown-item" data-action="prompt">&#9733; Show Prompt</button>
  `;
  dropdown.classList.add('open');

  dropdown.addEventListener('click', (e) => {
    const action = e.target.closest('[data-action]')?.dataset.action;
    closeProjectDropdown();
    if (action === 'edit') openProjectMenu();
    if (action === 'prompt') openPromptModal();
  });

  setTimeout(() => {
    document.addEventListener('click', closeProjectDropdown, { once: true });
  }, 0);
}

function closeProjectDropdown() {
  const dropdown = document.getElementById('project-dropdown');
  if (dropdown) {
    dropdown.classList.remove('open');
    dropdown.innerHTML = '';
  }
}

function openPromptModal() {
  if (!state.project) return;
  const prompt = generateProjectPrompt();
  document.getElementById('prompt-content').textContent = prompt;
  document.getElementById('prompt-modal').classList.add('open');
}

function closePromptModal() {
  document.getElementById('prompt-modal').classList.remove('open');
}

function generateProjectPrompt() {
  const p = state.project;
  const columns = (p.columns || []).filter(c => !c.hidden).sort((a, b) => a.order - b.order);
  const colList = columns.map(c => `  - id: "${c.id}", title: "${c.title}"`).join('\n');
  const existingTasks = (p.tasks || []).slice(0, 3);
  const taskExample = existingTasks.length > 0
    ? JSON.stringify(existingTasks[0], null, 2)
    : JSON.stringify({
        id: '',
        title: 'Beispiel-Task',
        description: 'Beschreibung des Tasks',
        column_id: columns[0]?.id || '',
        labels: ['feature'],
        order: 0,
        points: 5,
        worker: '',
        creator: '',
        comments: [],
        logs: [],
      }, null, 2);

  return `Du bist ein Projektmanagement-Assistent. Generiere Tasks als JSON für das Kanban-Board "${p.title}".

## Projekt-Struktur

Das Projekt hat folgende Spalten:
${colList}

## Task-Format

Jeder Task ist ein JSON-Objekt mit dieser Struktur:
${taskExample}

### Feld-Beschreibung:
- id: Leer lassen ("") – wird vom Server generiert
- title: Kurzer, prägnanter Titel des Tasks
- description: Ausführliche Beschreibung, Akzeptanzkriterien, Details
- column_id: ID der Spalte, in der der Task erscheinen soll (siehe Spalten oben)
- labels: Array von Strings, z.B. ["feature"], ["bug"], ["refactor"], ["docs"]
- order: Position innerhalb der Spalte (0 = oben)
- points: Story Points / Aufwand (0–100), z.B. 1=trivial, 3=klein, 5=mittel, 8=groß, 13=sehr groß
- worker: Name der zugewiesenen Person (leer lassen wenn unklar)
- creator: Name des Erstellers (leer lassen)
- comments: Array von Strings für Kommentare
- logs: Array von Strings für Logs (leer lassen)

## Antwort-Format

Antworte mit einem JSON-Array von Tasks:
[
  { "id": "", "title": "...", "description": "...", "column_id": "${columns[0]?.id || 'SPALTEN_ID'}", "labels": [...], "order": 0, "points": 5, "worker": "", "creator": "", "comments": [], "logs": [] },
  ...
]

## Aktuelle Tasks im Projekt (${(p.tasks || []).length} Stück):
${(p.tasks || []).length > 0 ? (p.tasks || []).map(t => `- [${columnName(t.column_id)}] ${t.title}`).join('\n') : '(keine)'}

Generiere jetzt Tasks basierend auf der folgenden Anforderung:
`;
}

async function openProjectMenu() {
  if (!state.project) return;
  const project = await api.get(`/api/projects/${state.project._id}?include_archived=true`);
  document.getElementById('proj-modal-title').value = project.title || '';
  document.getElementById('proj-modal-json').value = JSON.stringify(project, null, 2);
  renderJsonTree(project, document.getElementById('proj-json-tree'));
  document.getElementById('proj-json-tree').style.display = '';
  document.getElementById('proj-modal-json').style.display = 'none';
  const toggleBtn = document.getElementById('proj-view-toggle');
  toggleBtn.textContent = 'Raw JSON';
  document.getElementById('project-modal').classList.add('open');
}

function closeProjectMenu() {
  document.getElementById('project-modal').classList.remove('open');
}

async function copyProjectJson() {
  const textarea = document.getElementById('proj-modal-json');
  try {
    await navigator.clipboard.writeText(textarea.value);
    const btn = document.getElementById('proj-modal-copy');
    btn.textContent = 'Kopiert!';
    setTimeout(() => { btn.textContent = 'In Zwischenablage kopieren'; }, 1500);
  } catch {
    textarea.select();
  }
}

async function importProjectJson() {
  const text = document.getElementById('proj-modal-json').value.trim();
  if (!text) return;
  let data;
  try {
    data = JSON.parse(text);
  } catch {
    alert('Ungültiges JSON');
    return;
  }
  if (!confirm('Neues Projekt aus diesem JSON erstellen?')) return;
  data._id = '';
  delete data._rev;
  data.title = data.title ? data.title + ' (Import)' : 'Import';
  state.project = await api.post('/api/projects', data);
  await loadProjects();
  closeProjectMenu();
  renderBoard();
  updateProjectTitle();
  subscribeSSE(state.project._id);
}

// ------------------------------------------------------------------
// Projekt JSON Tree-View
// ------------------------------------------------------------------

function renderJsonTree(obj, container, depth = 0) {
  container.innerHTML = '';
  buildTreeNode(obj, container, depth, '');
}

function buildTreeNode(value, parent, depth, key) {
  if (value === null || value === undefined) {
    const line = document.createElement('div');
    line.className = 'json-line';
    line.style.paddingLeft = (depth * 16) + 'px';
    line.innerHTML = (key ? `<span class="json-key">${escapeHtml(key)}</span>: ` : '')
      + `<span class="json-value json-null">null</span>`;
    parent.appendChild(line);
    return;
  }

  if (Array.isArray(value)) {
    const wrapper = document.createElement('div');
    wrapper.className = 'json-node';

    const toggle = document.createElement('div');
    toggle.className = 'json-line json-toggle';
    toggle.style.paddingLeft = (depth * 16) + 'px';
    const collapsed = depth > 0;
    toggle.innerHTML = `<span class="json-arrow${collapsed ? '' : ' json-arrow-open'}">\u25B6</span>`
      + (key ? `<span class="json-key">${escapeHtml(key)}</span>: ` : '')
      + `<span class="json-bracket">[</span>`
      + `<span class="json-collapsed-hint">${collapsed ? value.length + ' items' : ''}</span>`;

    const children = document.createElement('div');
    children.className = 'json-children';
    if (collapsed) children.style.display = 'none';

    value.forEach((item, i) => {
      buildTreeNode(item, children, depth + 1, String(i));
    });

    const closeBracket = document.createElement('div');
    closeBracket.className = 'json-line';
    closeBracket.style.paddingLeft = (depth * 16) + 'px';
    closeBracket.innerHTML = '<span class="json-bracket">]</span>';
    if (collapsed) closeBracket.style.display = 'none';

    toggle.addEventListener('click', () => {
      const isHidden = children.style.display === 'none';
      children.style.display = isHidden ? '' : 'none';
      closeBracket.style.display = isHidden ? '' : 'none';
      toggle.querySelector('.json-arrow').classList.toggle('json-arrow-open', isHidden);
      toggle.querySelector('.json-collapsed-hint').textContent = isHidden ? '' : value.length + ' items';
    });

    wrapper.appendChild(toggle);
    wrapper.appendChild(children);
    wrapper.appendChild(closeBracket);
    parent.appendChild(wrapper);
    return;
  }

  if (typeof value === 'object') {
    const keys = Object.keys(value);
    const wrapper = document.createElement('div');
    wrapper.className = 'json-node';

    const toggle = document.createElement('div');
    toggle.className = 'json-line json-toggle';
    toggle.style.paddingLeft = (depth * 16) + 'px';
    const collapsed = depth > 0;
    toggle.innerHTML = `<span class="json-arrow${collapsed ? '' : ' json-arrow-open'}">\u25B6</span>`
      + (key ? `<span class="json-key">${escapeHtml(key)}</span>: ` : '')
      + `<span class="json-bracket">{</span>`
      + `<span class="json-collapsed-hint">${collapsed ? keys.length + ' keys' : ''}</span>`;

    const children = document.createElement('div');
    children.className = 'json-children';
    if (collapsed) children.style.display = 'none';

    keys.forEach(k => {
      buildTreeNode(value[k], children, depth + 1, k);
    });

    const closeBracket = document.createElement('div');
    closeBracket.className = 'json-line';
    closeBracket.style.paddingLeft = (depth * 16) + 'px';
    closeBracket.innerHTML = '<span class="json-bracket">}</span>';
    if (collapsed) closeBracket.style.display = 'none';

    toggle.addEventListener('click', () => {
      const isHidden = children.style.display === 'none';
      children.style.display = isHidden ? '' : 'none';
      closeBracket.style.display = isHidden ? '' : 'none';
      toggle.querySelector('.json-arrow').classList.toggle('json-arrow-open', isHidden);
      toggle.querySelector('.json-collapsed-hint').textContent = isHidden ? '' : keys.length + ' keys';
    });

    wrapper.appendChild(toggle);
    wrapper.appendChild(children);
    wrapper.appendChild(closeBracket);
    parent.appendChild(wrapper);
    return;
  }

  // Primitive values
  const line = document.createElement('div');
  line.className = 'json-line';
  line.style.paddingLeft = (depth * 16) + 'px';
  let cls = 'json-value';
  if (typeof value === 'string') cls += ' json-string';
  else if (typeof value === 'number') cls += ' json-number';
  else if (typeof value === 'boolean') cls += ' json-bool';

  const displayVal = typeof value === 'string' ? `"${escapeHtml(value)}"` : String(value);
  line.innerHTML = (key ? `<span class="json-key">${escapeHtml(key)}</span>: ` : '')
    + `<span class="${cls}">${displayVal}</span>`;
  parent.appendChild(line);
}

function toggleJsonView() {
  const tree = document.getElementById('proj-json-tree');
  const textarea = document.getElementById('proj-modal-json');
  const btn = document.getElementById('proj-view-toggle');
  if (textarea.style.display === 'none') {
    textarea.style.display = '';
    tree.style.display = 'none';
    btn.textContent = 'Tree';
  } else {
    // Re-parse textarea content to update tree
    try {
      const data = JSON.parse(textarea.value);
      renderJsonTree(data, tree);
    } catch { /* keep old tree */ }
    textarea.style.display = 'none';
    tree.style.display = '';
    btn.textContent = 'Raw JSON';
  }
}

// ------------------------------------------------------------------
// Projekt speichern (überschreiben)
// ------------------------------------------------------------------

async function saveProjectJson() {
  if (!state.project) return;
  const textarea = document.getElementById('proj-modal-json');
  const titleInput = document.getElementById('proj-modal-title');
  const text = textarea.value.trim();
  if (!text) return;

  let data;
  try {
    data = JSON.parse(text);
  } catch {
    alert('Ungültiges JSON');
    return;
  }

  // Titel aus dem Input-Feld übernehmen, falls geändert
  const newTitle = titleInput.value.trim();
  if (newTitle) data.title = newTitle;

  // _id und _rev beibehalten
  data._id = state.project._id;
  data._rev = state.project._rev;

  if (!confirm('Projekt mit diesem JSON überschreiben?')) return;

  try {
    state.project = await api.put(`/api/projects/${state.project._id}`, data);
    await loadProjects();
    closeProjectMenu();
    renderBoard();
    updateProjectTitle();
  } catch (err) {
    alert('Fehler beim Speichern: ' + err.message);
  }
}

async function saveProjectTitle() {
  if (!state.project) return;
  const titleInput = document.getElementById('proj-modal-title');
  const newTitle = titleInput.value.trim();
  if (newTitle && newTitle !== state.project.title) {
    await renameProject(state.project._id, newTitle);
  }
}

// ------------------------------------------------------------------
// Admin-Modal (Nutzerverwaltung)
// ------------------------------------------------------------------

let adminState = { users: [], editingUser: null, tokens: [], tab: 'users' };

async function openAdminModal() {
  adminState.tab = 'users';
  try {
    const r = await fetch('/api/admin/users');
    if (!r.ok) return;
    adminState.users = await r.json();
  } catch { return; }
  adminState.editingUser = null;
  updateAdminTabs();
  renderAdminUserList();
  document.getElementById('admin-user-form').style.display = 'none';
  document.getElementById('admin-user-list').style.display = '';
  document.getElementById('admin-list-actions').style.display = '';
  document.getElementById('admin-token-section').style.display = 'none';
  document.getElementById('admin-modal').classList.add('open');
}

function updateAdminTabs() {
  document.querySelectorAll('.admin-tab').forEach(t => {
    t.classList.toggle('admin-tab-active', t.dataset.tab === adminState.tab);
  });
}

async function switchAdminTab(tab) {
  adminState.tab = tab;
  updateAdminTabs();
  if (tab === 'users') {
    document.getElementById('admin-user-list').style.display = '';
    document.getElementById('admin-list-actions').style.display = '';
    document.getElementById('admin-user-form').style.display = 'none';
    document.getElementById('admin-token-section').style.display = 'none';
    renderAdminUserList();
  } else if (tab === 'tokens') {
    document.getElementById('admin-user-list').style.display = 'none';
    document.getElementById('admin-list-actions').style.display = 'none';
    document.getElementById('admin-user-form').style.display = 'none';
    document.getElementById('admin-token-section').style.display = '';
    await loadTokens();
  }
}

async function loadTokens() {
  try {
    const r = await fetch('/api/admin/tokens');
    if (!r.ok) return;
    adminState.tokens = await r.json();
  } catch { return; }
  renderTokenList();
}

function renderTokenList() {
  const el = document.getElementById('admin-token-list');
  if (adminState.tokens.length === 0) {
    el.innerHTML = '<div class="modal-list-empty">Keine Tokens</div>';
  } else {
    el.innerHTML = adminState.tokens.map(t => `
      <div class="admin-user-row">
        <span class="admin-user-name">${escapeHtml(t.name)}</span>
        <span class="admin-user-detail">${t.role} ${t.active === false ? '&middot; inaktiv' : ''}</span>
        <div class="admin-user-actions">
          <button class="btn-small" data-token-action="toggle" data-tid="${t.id}">${t.active ? 'Deaktivieren' : 'Aktivieren'}</button>
          <button class="btn-small btn-danger-small" data-token-action="delete" data-tid="${t.id}">L&ouml;schen</button>
        </div>
      </div>
    `).join('');
  }
}

async function createToken() {
  const name = document.getElementById('admin-token-name').value.trim();
  const role = document.getElementById('admin-token-role').value;
  if (!name) return;
  try {
    const r = await fetch('/api/admin/tokens', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name, role }),
    });
    if (!r.ok) return;
    const data = await r.json();
    // Token einmalig anzeigen.
    document.getElementById('admin-token-result').textContent = data.token;
    document.getElementById('admin-token-result').style.display = '';
    document.getElementById('admin-token-name').value = '';
    await loadTokens();
  } catch (err) {
    console.error('Token create error:', err);
  }
}

function closeAdminModal() {
  document.getElementById('admin-modal').classList.remove('open');
}

function renderAdminUserList() {
  const el = document.getElementById('admin-user-list');
  if (adminState.users.length === 0) {
    el.innerHTML = '<div class="modal-list-empty">Keine Nutzer</div>';
    return;
  }
  el.innerHTML = adminState.users.map(u => `
    <div class="admin-user-row">
      <span class="admin-user-name">${escapeHtml(u.display_name)}</span>
      <span class="admin-user-detail">${escapeHtml(u.username)} &middot; ${u.role}${u.active === false ? ' &middot; inaktiv' : ''}</span>
      <div class="admin-user-actions">
        <button class="btn-small" data-admin-action="edit" data-uid="${u.id}">Bearbeiten</button>
        <button class="btn-small" data-admin-action="reset-pw" data-uid="${u.id}">PW Reset</button>
        <button class="btn-small btn-danger-small" data-admin-action="delete" data-uid="${u.id}">L&ouml;schen</button>
      </div>
    </div>
  `).join('');
}

function showAdminForm(user) {
  adminState.editingUser = user || null;
  document.getElementById('admin-user-list').style.display = 'none';
  document.getElementById('admin-list-actions').style.display = 'none';
  document.getElementById('admin-user-form').style.display = '';
  document.getElementById('admin-username').value = user ? user.username : '';
  document.getElementById('admin-username').disabled = !!user;
  document.getElementById('admin-displayname').value = user ? user.display_name : '';
  document.getElementById('admin-password').value = '';
  document.getElementById('admin-password').placeholder = user ? '(unverändert)' : 'Passwort';
  document.getElementById('admin-role').value = user ? user.role : 'user';
  setTimeout(() => document.getElementById(user ? 'admin-displayname' : 'admin-username').focus(), 50);
}

async function saveAdminForm() {
  const username = document.getElementById('admin-username').value.trim();
  const displayName = document.getElementById('admin-displayname').value.trim();
  const password = document.getElementById('admin-password').value;
  const role = document.getElementById('admin-role').value;
  if (!username || !displayName) return;

  try {
    if (adminState.editingUser) {
      await fetch(`/api/admin/users/${adminState.editingUser.id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ display_name: displayName, role, active: true }),
      });
    } else {
      if (!password) return;
      await fetch('/api/admin/users', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ username, display_name: displayName, password, role }),
      });
    }
    await openAdminModal();
  } catch (err) {
    console.error('Admin save error:', err);
  }
}

// ------------------------------------------------------------------
// Passwort-Ändern Modal
// ------------------------------------------------------------------

function openPasswordModal(force) {
  document.getElementById('pw-error').textContent = '';
  document.getElementById('pw-old').value = '';
  document.getElementById('pw-new').value = '';
  document.getElementById('pw-confirm').value = '';
  const closeBtn = document.getElementById('pw-modal-close');
  closeBtn.style.display = force ? 'none' : '';
  document.getElementById('password-modal').dataset.force = force ? '1' : '';
  document.getElementById('password-modal').classList.add('open');
  setTimeout(() => document.getElementById('pw-old').focus(), 50);
}

function closePasswordModal() {
  if (document.getElementById('password-modal').dataset.force === '1') return;
  document.getElementById('password-modal').classList.remove('open');
}

async function savePassword() {
  const oldPw = document.getElementById('pw-old').value;
  const newPw = document.getElementById('pw-new').value;
  const confirmPw = document.getElementById('pw-confirm').value;
  const errEl = document.getElementById('pw-error');
  errEl.textContent = '';

  if (newPw !== confirmPw) {
    errEl.textContent = 'Passwörter stimmen nicht überein';
    return;
  }
  if (newPw.length < 4) {
    errEl.textContent = 'Passwort muss mindestens 4 Zeichen haben';
    return;
  }
  try {
    await doChangePassword(oldPw, newPw);
    document.getElementById('password-modal').dataset.force = '';
    document.getElementById('password-modal').classList.remove('open');
    const user = await checkAuth();
    if (user) {
      state.currentUser = user;
      updateUserSection();
    }
  } catch (err) {
    errEl.textContent = err.message;
  }
}

// ------------------------------------------------------------------
// SSE
// ------------------------------------------------------------------

function subscribeSSE(projectId) {
  if (state.eventSource) {
    state.eventSource.close();
    state.eventSource = null;
  }
  const es = new EventSource(`/api/projects/${projectId}/events`);
  es.addEventListener('project_update', async () => {
    // Während eines Drags nicht neu rendern – das würde den Drag abbrechen.
    if (state.isDragging) return;
    state.project = await api.get(`/api/projects/${projectId}`);
    renderBoard();
  });
  state.eventSource = es;
}

// ------------------------------------------------------------------
// Task-Edit-Modal
// ------------------------------------------------------------------

/** Modal für einen NEUEN Task öffnen (wird erst bei Save erstellt). */
function openNewTaskModal(columnId) {
  const newTask = {
    id: '',
    title: '',
    description: '',
    column_id: columnId,
    previous_row: '',
    assignee_ids: [],
    labels: [],
    order: tasksForColumn(columnId).length,
    points: 0,
    worker: '',
    creator: '',
    logs: [],
    comments: [],
    created_at: '',
    updated_at: '',
  };
  openTaskModal(newTask, true);
}

function openTaskModal(task, isNew) {
  state.editingTask = { ...task, logs: [...(task.logs || [])], comments: [...(task.comments || [])] };
  state.isNewTask = !!isNew;

  document.getElementById('modal-heading-text').textContent = isNew ? 'Neuer Task' : 'Task bearbeiten';
  document.getElementById('modal-title').value = task.title;
  document.getElementById('modal-desc').value = task.description || '';
  document.getElementById('modal-labels').value = (task.labels || []).join(', ');
  document.getElementById('modal-points').value = task.points || 0;
  document.getElementById('modal-worker').value = task.worker || (isNew && state.currentUser ? state.currentUser.display_name : '');
  document.getElementById('modal-created').textContent = formatDate(task.created_at);
  document.getElementById('modal-updated').textContent = formatDate(task.updated_at);
  document.getElementById('modal-prev-row').textContent = columnName(task.previous_row);

  // Seitenleiste bei neuem Task ausblenden (noch keine Metadaten).
  document.querySelector('.modal-col-side').style.display = isNew ? 'none' : '';
  document.getElementById('modal-delete-btn').style.display = isNew ? 'none' : '';

  const logsEl = document.getElementById('modal-logs');
  logsEl.innerHTML = (task.logs || []).length
      ? [...(task.logs || [])].reverse().map(l => `<div class="modal-list-item">${escapeHtml(l)}</div>`).join('')
      : '<div class="modal-list-empty">Keine Logs</div>';

  renderModalComments();
  document.getElementById('task-modal').classList.add('open');

  // Titel-Feld fokussieren bei neuem Task.
  if (isNew) {
    setTimeout(() => document.getElementById('modal-title').focus(), 50);
  }
}

function renderModalComments() {
  const el = document.getElementById('modal-comments');
  const comments = state.editingTask?.comments || [];
  el.innerHTML = comments.length
      ? comments.map(c => `<div class="modal-list-item">${escapeHtml(c)}</div>`).join('')
      : '<div class="modal-list-empty">Keine Kommentare</div>';
}

function closeTaskModal() {
  document.getElementById('task-modal').classList.remove('open');
  state.editingTask = null;
  state.isNewTask = false;
}

// ------------------------------------------------------------------
// Task Detail-Ansicht
// ------------------------------------------------------------------

function openTaskDetail(task) {
  state.detailTask = task;

  document.getElementById('detail-title').textContent = task.title || 'Untitled';

  // Current column badge
  const col = (state.project?.columns || []).find(c => c.id === task.column_id);
  const colColor = col?.color || 'var(--accent)';
  document.getElementById('detail-column-info').innerHTML = col
      ? `<span class="detail-column-badge"><span class="detail-column-dot" style="background:${colColor}"></span>${escapeHtml(col.title)}</span>`
      : '';

  // Description
  const descEl = document.getElementById('detail-description');
  if (task.description) {
    descEl.textContent = task.description;
    descEl.classList.remove('detail-description-empty');
  } else {
    descEl.textContent = 'Keine Beschreibung';
    descEl.classList.add('detail-description-empty');
  }

  // Labels
  const labelsEl = document.getElementById('detail-labels');
  labelsEl.innerHTML = (task.labels || []).length
      ? (task.labels || []).map(l => `<span class="detail-label">${escapeHtml(l)}</span>`).join('')
      : '<span class="detail-empty">Keine Labels</span>';

  // Points
  document.getElementById('detail-points').innerHTML = task.points
      ? `<span class="detail-points-badge">${task.points}</span>`
      : '–';

  // Worker
  const workerEl = document.getElementById('detail-worker');
  if (task.worker) {
    workerEl.innerHTML = `<div class="detail-worker"><span class="detail-worker-avatar">${escapeHtml(task.worker[0].toUpperCase())}</span><span class="detail-worker-name">${escapeHtml(task.worker)}</span></div>`;
  } else {
    workerEl.textContent = '–';
  }

  // Dates
  document.getElementById('detail-created').textContent = formatDate(task.created_at);
  document.getElementById('detail-updated').textContent = formatDate(task.updated_at);

  // Previous row
  document.getElementById('detail-prev-row').textContent = columnName(task.previous_row);

  // Comments
  const commentsEl = document.getElementById('detail-comments');
  commentsEl.innerHTML = (task.comments || []).length
      ? (task.comments || []).map(c => `<div class="detail-list-item">${escapeHtml(c)}</div>`).join('')
      : '<div class="detail-empty">Keine Kommentare</div>';

  // Logs
  const logsEl = document.getElementById('detail-logs');
  logsEl.innerHTML = (task.logs || []).length
      ? [...(task.logs || [])].reverse().map(l => `<div class="detail-log-item">${escapeHtml(l)}</div>`).join('')
      : '<div class="detail-empty">Keine Logs</div>';

  document.getElementById('task-detail-modal').classList.add('open');
}

function closeTaskDetail() {
  document.getElementById('task-detail-modal').classList.remove('open');
  state.detailTask = null;
}

// ------------------------------------------------------------------
// UI-Helfer
// ------------------------------------------------------------------

function renderProjectList() {
  const list = document.getElementById('project-list');
  list.innerHTML = '';
  state.projects.forEach(p => {
    const li = document.createElement('li');
    li.className = 'project-item' + (p._id === state.project?._id ? ' active' : '');
    li.dataset.id = p._id;

    const nameSpan = document.createElement('span');
    nameSpan.className = 'project-name';
    nameSpan.textContent = p.title;
    nameSpan.addEventListener('click', () => openProject(p._id));

    li.appendChild(nameSpan);

    if (state.projects.length > 1) {
      const delBtn = document.createElement('button');
      delBtn.className = 'project-delete-btn';
      delBtn.textContent = '×';
      delBtn.title = 'Projekt löschen';
      delBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        if (confirm(`Projekt "${p.title}" und alle Tasks wirklich löschen?`)) deleteProject(p._id);
      });
      li.appendChild(delBtn);
    }

    list.appendChild(li);
  });
}

function updateProjectTitle() {
  const el = document.getElementById('project-title');
  if (el) el.textContent = state.project?.title || '';
}

// ------------------------------------------------------------------
// DOM aufbauen
// ------------------------------------------------------------------

function buildDOM() {
  document.body.innerHTML = `
    <div class="app">
      <aside class="sidebar">
        <div class="sidebar-header">
          <span class="logo"><img src="/icons/favicon-32.png" alt="" class="logo-icon" /> Plankton</span>
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

  // Board: Delegierter Click-Handler (einmal registriert, nicht bei jedem renderBoard).
  document.getElementById('board').addEventListener('click', e => {
    // Während eines Drags keine Klicks verarbeiten.
    if (state.isDragging) return;

    // Checkbox: Task selektieren/deselektieren.
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

    // Task öffnen (aber nicht wenn Checkbox geklickt wurde).
    const inner = e.target.closest('[data-task-id]');
    if (inner && !e.target.closest('.task-checkbox')) {
      const task = state.project.tasks.find(t => t.id === inner.dataset.taskId);
      if (task) openTaskDetail(task);
      return;
    }

    // "+ Task"-Button.
    const addBtn = e.target.closest('.col-add-btn');
    if (addBtn) {
      openNewTaskModal(addBtn.dataset.colId);
      return;
    }

    // Spalten-Menü (Burger).
    const menuBtn = e.target.closest('.col-menu-btn');
    if (menuBtn) {
      e.stopPropagation();
      openColumnMenu(menuBtn, menuBtn.dataset.colId);
      return;
    }
  });

  // Detail-Ansicht schließen.
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

  // Modal schließen.
  document.getElementById('modal-close-btn').addEventListener('click', closeTaskModal);
  document.getElementById('task-modal').addEventListener('click', e => {
    if (e.target.id === 'task-modal') closeTaskModal();
  });

  // Modal speichern: neuen Task erstellen ODER bestehenden updaten.
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

  // Modal löschen.
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
    columnModalState.selectedColor = swatch.dataset.color;
    renderColorPicker(swatch.dataset.color);
  });

  // Projekt-Menü (Dropdown).
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
  document.getElementById('logout-btn').addEventListener('click', doLogout);
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
    const action = btn.dataset.tokenAction;
    const tid = btn.dataset.tid;
    if (action === 'delete') {
      if (!confirm('Token wirklich löschen?')) return;
      await fetch(`/api/admin/tokens/${tid}`, { method: 'DELETE' });
      await loadTokens();
    } else if (action === 'toggle') {
      const token = adminState.tokens.find(t => t.id === tid);
      if (!token) return;
      await fetch(`/api/admin/tokens/${tid}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ active: !token.active }),
      });
      await loadTokens();
    }
  });
  document.getElementById('admin-user-list').addEventListener('click', async (e) => {
    const btn = e.target.closest('[data-admin-action]');
    if (!btn) return;
    const action = btn.dataset.adminAction;
    const uid = btn.dataset.uid;
    if (action === 'edit') {
      const user = adminState.users.find(u => u.id === uid);
      if (user) showAdminForm(user);
    } else if (action === 'delete') {
      if (!confirm('Nutzer wirklich löschen?')) return;
      await fetch(`/api/admin/users/${uid}`, { method: 'DELETE' });
      await openAdminModal();
    } else if (action === 'reset-pw') {
      const newPw = prompt('Neues Passwort:');
      if (!newPw) return;
      await fetch(`/api/admin/users/${uid}/reset-password`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ new_password: newPw }),
      });
      alert('Passwort zurückgesetzt');
    }
  });

  // Passwort-Modal.
  document.getElementById('pw-modal-close').addEventListener('click', closePasswordModal);
  document.getElementById('password-modal').addEventListener('click', e => {
    if (e.target.id === 'password-modal') closePasswordModal();
  });
  document.getElementById('pw-save-btn').addEventListener('click', savePassword);
}

// ------------------------------------------------------------------
// Bootstrap
// ------------------------------------------------------------------

async function init() {
  const user = await checkAuth();
  if (!user) {
    showLoginPage();
    return;
  }
  state.currentUser = user;
  await startApp();
  if (user.must_change_password) {
    setTimeout(() => openPasswordModal(true), 100);
  }
}

document.addEventListener('DOMContentLoaded', init);
