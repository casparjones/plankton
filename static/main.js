import './styles.css';
import jKanban from 'jkanban';

// ============================================================
// Plankton Frontend – jKanban + Vanilla JS
// Kein Framework-Overhead. Jede Änderung am Board wird sofort
// per REST-API an das Rust-Backend persistiert.
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
  projects: [],       // Projektliste für die Sidebar
  project: null,      // Aktives Projekt (vollständiges ProjectDoc)
  kanban: null,       // jKanban-Instanz
  editingTask: null,  // Task der gerade im Modal bearbeitet wird
  eventSource: null,  // SSE-Verbindung für Live-Updates
};

// ------------------------------------------------------------------
// Hilfsfunktionen
// ------------------------------------------------------------------

/** Gibt alle Tasks einer Spalte sortiert nach `order` zurück. */
function tasksForColumn(columnId) {
  return (state.project?.tasks || [])
      .filter(t => t.column_id === columnId)
      .sort((a, b) => a.order - b.order);
}

/** Wandelt einen Task in das jKanban-Item-Format um. */
function taskToItem(task) {
  const assignees = (task.assignee_ids || [])
      .map(id => state.project.users.find(u => u.id === id))
      .filter(Boolean)
      .map(u => `<span class="avatar" title="${u.name}">${u.name[0]}</span>`)
      .join('');

  const labels = (task.labels || [])
      .map(l => `<span class="label">${escapeHtml(l)}</span>`)
      .join('');

  return {
    id: task.id,
    title: `
      <div class="task-inner" data-task-id="${task.id}">
        <div class="task-title">${escapeHtml(task.title)}</div>
        ${task.description
        ? `<div class="task-desc">${escapeHtml(task.description.substring(0, 80))}${task.description.length > 80 ? '…' : ''}</div>`
        : ''}
        <div class="task-meta">
          <div class="task-labels">${labels}</div>
          <div class="task-assignees">${assignees}</div>
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

// ------------------------------------------------------------------
// Board rendern
// ------------------------------------------------------------------

/** Zerstört die alte jKanban-Instanz und baut das Board neu auf. */
function renderBoard() {
  if (!state.project) return;

  // Altes Board aufräumen
  if (state.kanban) {
    try { state.kanban.destroy(); } catch (_) {}
    state.kanban = null;
  }

  const container = document.getElementById('board');
  container.innerHTML = '';

  const columns = [...(state.project.columns || [])].sort((a, b) => a.order - b.order);

  const boards = columns.map(col => ({
    id: col.id,
    title: `
      <span class="col-title" style="border-color:${col.color}">${escapeHtml(col.title)}</span>
      <button class="col-add-btn" data-col-id="${col.id}" title="Task hinzufügen">+</button>
    `,
    item: tasksForColumn(col.id).map(taskToItem),
    class: 'kanban-column',
  }));

  state.kanban = new jKanban({
    element: '#board',
    gutter: '0',
    widthBoard: '300px',
    responsivePercentage: false,
    dragBoards: false,
    addItemButton: false,
    boards,

    dragEl(el) {
      el.classList.add('dragging');
    },
    dragendEl(el) {
      el.classList.remove('dragging');
    },
    dropEl(el, target) {
      const taskId = el.querySelector('[data-task-id]')?.dataset.taskId;
      const columnId = target.closest('.kanban-board')?.dataset.id;
      if (!taskId || !columnId) return;

      const items = [...target.querySelectorAll('[data-task-id]')];
      const newOrder = items.findIndex(i => i.dataset.taskId === taskId);

      moveTask(taskId, columnId, Math.max(0, newOrder));
    },
  });

  // Delegierter Click-Handler: Tasks öffnen + "+ Task"-Button
  container.addEventListener('click', e => {
    const inner = e.target.closest('[data-task-id]');
    if (inner) {
      const task = state.project.tasks.find(t => t.id === inner.dataset.taskId);
      if (task) openTaskModal(task);
      return;
    }
    const addBtn = e.target.closest('.col-add-btn');
    if (addBtn) {
      createTask(addBtn.dataset.colId);
    }
  });
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
  renderBoard();
  updateProjectTitle();
  subscribeSSE(id);
}

async function createProject(title) {
  const payload = {
    _id: '',
    title: title || 'Untitled',
    columns: [
      { id: crypto.randomUUID(), title: 'Todo',        order: 0, color: '#90CAF9' },
      { id: crypto.randomUUID(), title: 'In Progress', order: 1, color: '#FFCC80' },
      { id: crypto.randomUUID(), title: 'Done',        order: 2, color: '#A5D6A7' },
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

async function createTask(columnId) {
  const task = {
    id: '',
    title: 'New Task',
    description: '',
    column_id: columnId,
    assignee_ids: [],
    labels: [],
    order: tasksForColumn(columnId).length,
    created_at: '',
    updated_at: '',
  };
  state.project = await api.post(`/api/projects/${state.project._id}/tasks`, task);
  renderBoard();
}

async function saveTask(task) {
  state.project = await api.put(
      `/api/projects/${state.project._id}/tasks/${task.id}`,
      task
  );
  renderBoard();
}

async function deleteTask(taskId) {
  await api.del(`/api/projects/${state.project._id}/tasks/${taskId}`);
  state.project = await api.get(`/api/projects/${state.project._id}`);
  renderBoard();
}

async function moveTask(taskId, columnId, order) {
  try {
    state.project = await api.post(
        `/api/projects/${state.project._id}/tasks/${taskId}/move`,
        { column_id: columnId, order }
    );
    renderBoard();
  } catch (err) {
    console.error('Move failed:', err);
  }
}

// ------------------------------------------------------------------
// SSE – Live-Updates wenn mehrere Clients offen sind
// ------------------------------------------------------------------

function subscribeSSE(projectId) {
  if (state.eventSource) {
    state.eventSource.close();
    state.eventSource = null;
  }
  const es = new EventSource(`/api/projects/${projectId}/events`);
  es.addEventListener('project_update', async () => {
    state.project = await api.get(`/api/projects/${projectId}`);
    renderBoard();
  });
  state.eventSource = es;
}

// ------------------------------------------------------------------
// Task-Edit-Modal
// ------------------------------------------------------------------

function openTaskModal(task) {
  state.editingTask = { ...task };
  document.getElementById('modal-title').value = task.title;
  document.getElementById('modal-desc').value = task.description || '';
  document.getElementById('modal-labels').value = (task.labels || []).join(', ');
  document.getElementById('task-modal').classList.add('open');
}

function closeTaskModal() {
  document.getElementById('task-modal').classList.remove('open');
  state.editingTask = null;
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
    li.textContent = p.title;
    li.dataset.id = p._id;
    li.addEventListener('click', () => openProject(p._id));
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
          <span class="logo">🪼 Plankton</span>
        </div>
        <div class="sidebar-create">
          <input id="new-project-input" placeholder="Projektname…" />
          <button id="new-project-btn">Erstellen</button>
        </div>
        <ul id="project-list" class="project-list"></ul>
      </aside>

      <main class="main">
        <header class="board-header">
          <h1 id="project-title" class="board-title"></h1>
        </header>
        <div id="board" class="board"></div>
      </main>
    </div>

    <div id="task-modal" class="modal-overlay">
      <div class="modal">
        <div class="modal-header">
          <span class="modal-heading">Task bearbeiten</span>
          <button class="modal-close" id="modal-close-btn">✕</button>
        </div>
        <label>Titel
          <input id="modal-title" type="text" />
        </label>
        <label>Beschreibung
          <textarea id="modal-desc" rows="5"></textarea>
        </label>
        <label>Labels <small>(kommagetrennt)</small>
          <input id="modal-labels" type="text" />
        </label>
        <div class="modal-actions">
          <button id="modal-save-btn" class="btn-primary">Speichern</button>
          <button id="modal-delete-btn" class="btn-danger">Löschen</button>
        </div>
      </div>
    </div>
  `;

  document.getElementById('new-project-btn').addEventListener('click', () => {
    const input = document.getElementById('new-project-input');
    createProject(input.value.trim());
    input.value = '';
  });

  document.getElementById('new-project-input').addEventListener('keydown', e => {
    if (e.key === 'Enter') document.getElementById('new-project-btn').click();
  });

  document.getElementById('modal-close-btn').addEventListener('click', closeTaskModal);

  document.getElementById('task-modal').addEventListener('click', e => {
    if (e.target.id === 'task-modal') closeTaskModal();
  });

  document.getElementById('modal-save-btn').addEventListener('click', () => {
    if (!state.editingTask) return;
    const task = {
      ...state.editingTask,
      title: document.getElementById('modal-title').value,
      description: document.getElementById('modal-desc').value,
      labels: document.getElementById('modal-labels').value
          .split(',').map(s => s.trim()).filter(Boolean),
    };
    saveTask(task);
    closeTaskModal();
  });

  document.getElementById('modal-delete-btn').addEventListener('click', () => {
    if (!state.editingTask) return;
    if (confirm(`Task "${state.editingTask.title}" wirklich löschen?`)) {
      deleteTask(state.editingTask.id);
      closeTaskModal();
    }
  });
}

// ------------------------------------------------------------------
// Bootstrap
// ------------------------------------------------------------------

async function init() {
  buildDOM();
  await loadProjects();
  if (state.projects.length > 0) {
    await openProject(state.projects[0]._id);
  }
}

document.addEventListener('DOMContentLoaded', init);