// Bulk-Aktions-Leiste für Mehrfachauswahl.

import api from '../api';
import { state } from '../state';
import { t } from '../i18n';
import { renderBoard } from './board';
import { toastConfirm } from '../toast';
import type { ProjectDoc } from '../types';

export function updateBulkBar(): void {
  const bar = document.getElementById('bulk-bar');
  if (!bar) return;
  const count = state.selectedTasks.size;
  if (count === 0) {
    bar.classList.remove('visible');
  } else {
    bar.classList.add('visible');
    document.getElementById('bulk-count')!.textContent = String(count);
  }
}

export async function bulkDeleteSelected(): Promise<void> {
  const ids = [...state.selectedTasks];
  if (ids.length === 0) return;
  if (!await toastConfirm(t('bulk.deleteConfirm', { count: ids.length }))) return;

  for (const taskId of ids) {
    try {
      await api.del(`/api/projects/${state.project!._id}/tasks/${taskId}`);
    } catch (err) {
      console.error(t('bulk.deleteError'), taskId, err);
    }
  }
  state.selectedTasks.clear();
  state.project = await api.get<ProjectDoc>(`/api/projects/${state.project!._id}`);
  renderBoard();
}
