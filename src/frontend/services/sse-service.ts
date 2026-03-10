// SSE (Server-Sent Events) für Echtzeit-Updates.

import api from '../api';
import { state } from '../state';
import { renderBoard } from '../components/board';
import type { ProjectDoc } from '../types';

export function subscribeSSE(projectId: string): void {
  if (state.eventSource) {
    state.eventSource.close();
    state.eventSource = null;
  }
  const es = new EventSource(`/api/projects/${projectId}/events`);
  es.addEventListener('project_update', async () => {
    if (state.isDragging) return;
    state.project = await api.get<ProjectDoc>(`/api/projects/${projectId}`);
    renderBoard();
  });
  state.eventSource = es;
}
