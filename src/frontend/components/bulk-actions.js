// Bulk-Aktions-Leiste für Mehrfachauswahl.

import api from '../api.js';
import { state } from '../state.js';
import { renderBoard } from './board.js';

export function updateBulkBar() {
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

export async function bulkDeleteSelected() {
  const ids = [...state.selectedTasks];
  if (ids.length === 0) return;
  if (!confirm(`${ids.length} Task(s) wirklich löschen?`)) return;

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
