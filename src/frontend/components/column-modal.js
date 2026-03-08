// Spalten-Management (CRUD, Modal, Kontextmenü, Reihenfolge).

import api from '../api.js';
import { state, COLUMN_COLORS } from '../state.js';
import { renderBoard } from './board.js';

let columnModalState = { mode: null, colId: null, selectedColor: null };

// -- Spalten-CRUD --

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
  const taskCount = (state.project?.tasks || []).filter(t => t.column_id === colId).length;
  const msg = taskCount > 0
      ? `Spalte "${col.title}" und ${taskCount} Task(s) wirklich löschen?`
      : `Spalte "${col.title}" wirklich löschen?`;
  if (!confirm(msg)) return;
  state.project = await api.del(`/api/projects/${state.project._id}/columns/${colId}`)
      .then(() => api.get(`/api/projects/${state.project._id}`));
  renderBoard();
}

// -- Spalten-Kontextmenü --

export function openColumnMenu(anchorEl, colId) {
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
    <button class="col-ctx-item${col.locked ? ' col-ctx-disabled' : ' col-ctx-danger'}" data-action="delete" ${col.locked ? 'disabled title="Diese Spalte kann nicht gelöscht werden"' : ''}>&#10005; Spalte löschen</button>
  `;

  menu.addEventListener('click', (e) => {
    const action = e.target.closest('[data-action]')?.dataset.action;
    closeColumnMenu();
    if (action === 'edit') openColumnEditModal(colId);
    if (action === 'add') openColumnAddModal();
    if (action === 'delete' && !col.locked) deleteColumn(colId);
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

// -- Spalten-Modal --

export function openColumnEditModal(colId) {
  const col = state.project.columns.find(c => c.id === colId);
  if (!col) return;
  columnModalState = { mode: 'edit', colId, selectedColor: col.color };
  document.getElementById('col-modal-heading').textContent = 'Spalte bearbeiten';
  document.getElementById('col-modal-title').value = col.title;
  renderColorPicker(col.color);
  document.getElementById('column-modal').classList.add('open');
  setTimeout(() => document.getElementById('col-modal-title').focus(), 50);
}

export function openColumnAddModal() {
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

export function closeColumnModal() {
  document.getElementById('column-modal').classList.remove('open');
  columnModalState = { mode: null, colId: null, selectedColor: null };
}

export async function saveColumnModal() {
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

export function selectColor(color) {
  columnModalState.selectedColor = color;
  renderColorPicker(color);
}

// -- Spalten verschieben --

export async function reorderColumnsFromDOM() {
  const boardEls = [...document.querySelectorAll('#board .kanban-board')];
  const idOrder = boardEls.map(b => b.dataset.id);

  const doneCol = state.project.columns.find(c => c.title === 'Done' && !c.hidden);
  if (doneCol) {
    const doneIdx = idOrder.indexOf(doneCol.id);
    if (doneIdx !== -1 && doneIdx !== idOrder.length - 1) {
      idOrder.splice(doneIdx, 1);
      idOrder.push(doneCol.id);
      renderBoard();
      return;
    }
  }

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
