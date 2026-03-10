// Board rendern – Bridge zu KanbanBoard.vue (VueDraggablePlus).
// Legacy-Module importieren weiterhin renderBoard() von hier.

import { state } from '../state';
import { updateBulkBar } from './bulk-actions';
import { updateGitStatusIcon } from './git-settings';

declare global {
  interface Window {
    __kanbanRefresh?: () => void;
  }
}

export function renderBoard(): void {
  if (!state.project) return;
  updateGitStatusIcon();
  updateBulkBar();

  // Vue KanbanBoard-Komponente aktualisieren (registriert sich via window.__kanbanRefresh).
  if (typeof window.__kanbanRefresh === 'function') {
    window.__kanbanRefresh();
  }
}
