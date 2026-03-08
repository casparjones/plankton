// Task-Edit-Modal.

import { state } from '../state.js';
import { escapeHtml, columnName, formatDate } from '../utils.js';

export function tasksForColumn(columnId) {
  return (state.project?.tasks || [])
      .filter(t => t.column_id === columnId)
      .sort((a, b) => a.order - b.order);
}

// Feste Farbzuweisung pro Worker (Hash → Farbpalette).
const WORKER_COLORS = [
  '#64B5F6', '#FFB74D', '#81C784', '#E57373',
  '#BA68C8', '#4DD0E1', '#FF8A65', '#AED581',
  '#F06292', '#7986CB',
];
const workerColorCache = {};
function workerBorderColor(worker) {
  if (!worker) return 'var(--border)';
  const key = worker.trim().toLowerCase();
  if (!workerColorCache[key]) {
    let hash = 0;
    for (let i = 0; i < key.length; i++) hash = ((hash << 5) - hash + key.charCodeAt(i)) | 0;
    workerColorCache[key] = WORKER_COLORS[Math.abs(hash) % WORKER_COLORS.length];
  }
  return workerColorCache[key];
}

export function taskToItem(task) {
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
  const borderColor = workerBorderColor(task.worker);

  return {
    id: task.id,
    title: `
      <div class="task-inner ${isSelected ? 'task-selected' : ''}" data-task-id="${task.id}" style="border-left: 3px solid ${borderColor}">
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

export function openNewTaskModal(columnId) {
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

export function openTaskModal(task, isNew) {
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

  document.querySelector('.modal-col-side').style.display = isNew ? 'none' : '';
  document.getElementById('modal-delete-btn').style.display = isNew ? 'none' : '';

  const logsEl = document.getElementById('modal-logs');
  logsEl.innerHTML = (task.logs || []).length
      ? [...(task.logs || [])].reverse().map(l => `<div class="modal-list-item">${escapeHtml(l)}</div>`).join('')
      : '<div class="modal-list-empty">Keine Logs</div>';

  renderModalComments();
  document.getElementById('task-modal').classList.add('open');

  if (isNew) {
    setTimeout(() => document.getElementById('modal-title').focus(), 50);
  }
}

export function renderModalComments() {
  const el = document.getElementById('modal-comments');
  const comments = state.editingTask?.comments || [];
  el.innerHTML = comments.length
      ? comments.map(c => `<div class="modal-list-item">${escapeHtml(c)}</div>`).join('')
      : '<div class="modal-list-empty">Keine Kommentare</div>';
}

export function closeTaskModal() {
  document.getElementById('task-modal').classList.remove('open');
  state.editingTask = null;
  state.isNewTask = false;
}
