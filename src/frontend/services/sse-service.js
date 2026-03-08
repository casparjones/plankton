// SSE (Server-Sent Events) für Echtzeit-Updates.

import api from '../api.js';
import { state } from '../state.js';
import { renderBoard } from '../components/board.js';

export function subscribeSSE(projectId) {
  if (state.eventSource) {
    state.eventSource.close();
    state.eventSource = null;
  }
  const es = new EventSource(`/api/projects/${projectId}/events`);
  es.addEventListener('project_update', async () => {
    if (state.isDragging) return;
    state.project = await api.get(`/api/projects/${projectId}`);
    renderBoard();
  });
  state.eventSource = es;
}
