// Board rendern (jKanban).

import jKanban from 'jkanban';
import { state } from '../state.js';
import { escapeHtml } from '../utils.js';
import { tasksForColumn, taskToItem } from './task-modal.js';
import { updateBulkBar } from './bulk-actions.js';
import { moveTask } from '../services/project-service.js';
import { reorderColumnsFromDOM } from './column-modal.js';
import { updateGitStatusIcon } from './git-settings.js';

export function renderBoard() {
  if (!state.project) return;
  updateGitStatusIcon();

  if (state.kanban) {
    try { state.kanban.destroy(); } catch (_) {}
    state.kanban = null;
  }

  const container = document.getElementById('board');
  container.innerHTML = '';

  updateBulkBar();

  const columns = [...(state.project.columns || [])]
      .filter(c => !c.hidden)
      .sort((a, b) => {
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
