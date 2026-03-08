// Task Detail-Ansicht (read-only).

import { state } from '../state.js';
import { escapeHtml, columnName, formatDate } from '../utils.js';

export function openTaskDetail(task) {
  state.detailTask = task;

  document.getElementById('detail-title').textContent = task.title || 'Untitled';

  const col = (state.project?.columns || []).find(c => c.id === task.column_id);
  const colColor = col?.color || 'var(--accent)';
  document.getElementById('detail-column-info').innerHTML = col
      ? `<span class="detail-column-badge"><span class="detail-column-dot" style="background:${colColor}"></span>${escapeHtml(col.title)}</span>`
      : '';

  const descEl = document.getElementById('detail-description');
  if (task.description) {
    descEl.textContent = task.description;
    descEl.classList.remove('detail-description-empty');
  } else {
    descEl.textContent = 'Keine Beschreibung';
    descEl.classList.add('detail-description-empty');
  }

  const labelsEl = document.getElementById('detail-labels');
  labelsEl.innerHTML = (task.labels || []).length
      ? (task.labels || []).map(l => `<span class="detail-label">${escapeHtml(l)}</span>`).join('')
      : '<span class="detail-empty">Keine Labels</span>';

  document.getElementById('detail-points').innerHTML = task.points
      ? `<span class="detail-points-badge">${task.points}</span>`
      : '–';

  const workerEl = document.getElementById('detail-worker');
  if (task.worker) {
    workerEl.innerHTML = `<div class="detail-worker"><span class="detail-worker-avatar">${escapeHtml(task.worker[0].toUpperCase())}</span><span class="detail-worker-name">${escapeHtml(task.worker)}</span></div>`;
  } else {
    workerEl.textContent = '–';
  }

  document.getElementById('detail-created').textContent = formatDate(task.created_at);
  document.getElementById('detail-updated').textContent = formatDate(task.updated_at);
  document.getElementById('detail-prev-row').textContent = columnName(task.previous_row);

  const commentsEl = document.getElementById('detail-comments');
  commentsEl.innerHTML = (task.comments || []).length
      ? (task.comments || []).map(c => `<div class="detail-list-item">${escapeHtml(c)}</div>`).join('')
      : '<div class="detail-empty">Keine Kommentare</div>';

  const logsEl = document.getElementById('detail-logs');
  logsEl.innerHTML = (task.logs || []).length
      ? [...(task.logs || [])].reverse().map(l => `<div class="detail-log-item">${escapeHtml(l)}</div>`).join('')
      : '<div class="detail-empty">Keine Logs</div>';

  document.getElementById('task-detail-modal').classList.add('open');
}

export function closeTaskDetail() {
  document.getElementById('task-detail-modal').classList.remove('open');
  state.detailTask = null;
}
