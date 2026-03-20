// SSE (Server-Sent Events) für Echtzeit-Updates.
// Verarbeitet granulare Events (task_created, task_updated, task_moved, task_deleted)
// und fällt auf Full-Refetch zurück bei project_update oder unbekannten Events.

import api from '../api';
import { state } from '../state';
import { renderBoard } from '../components/board';
import type { ProjectDoc, Task } from '../types';

interface SSEPayload {
  event: string;
  data: Record<string, unknown>;
}

/** Task im lokalen State patchen (ersetzen oder hinzufügen). */
function patchTask(task: Task): void {
  if (!state.project) return;
  const idx = state.project.tasks.findIndex((t: Task) => t.id === task.id);
  if (idx >= 0) {
    state.project.tasks[idx] = task;
  } else {
    state.project.tasks.push(task);
  }
  // Detail-Ansicht aktualisieren falls derselbe Task offen ist
  if (state.detailTask && state.detailTask.id === task.id) {
    Object.assign(state.detailTask, task);
  }
}

/** Task aus lokalem State entfernen. */
function removeTask(taskId: string): void {
  if (!state.project) return;
  state.project.tasks = state.project.tasks.filter((t: Task) => t.id !== taskId);
  // Detail-Ansicht schließen falls gelöschter Task offen ist
  if (state.detailTask && state.detailTask.id === taskId) {
    state.detailTask = null;
  }
}

export function subscribeSSE(projectId: string): void {
  if (state.eventSource) {
    state.eventSource.close();
    state.eventSource = null;
  }
  const es = new EventSource(`/api/projects/${projectId}/events`);

  // Granulare Events
  es.addEventListener('project_event', async (e: MessageEvent) => {
    if (state.isDragging) return;

    let payload: SSEPayload;
    try {
      payload = JSON.parse(e.data);
    } catch {
      // Fallback: altes Format (plain project_id string)
      state.project = await api.get<ProjectDoc>(`/api/projects/${projectId}`);
      renderBoard();
      return;
    }

    switch (payload.event) {
      case 'task_created':
      case 'task_updated':
      case 'task_moved':
        patchTask(payload.data as unknown as Task);
        renderBoard();
        break;

      case 'task_deleted':
        removeTask((payload.data as { task_id: string }).task_id);
        renderBoard();
        break;

      case 'project_update':
      default:
        // Full-Refetch für Projekt-Level-Änderungen (Spalten, User, etc.)
        state.project = await api.get<ProjectDoc>(`/api/projects/${projectId}`);
        renderBoard();
        break;
    }
  });

  // Legacy-Kompatibilität: altes Event-Format
  es.addEventListener('project_update', async () => {
    if (state.isDragging) return;
    state.project = await api.get<ProjectDoc>(`/api/projects/${projectId}`);
    renderBoard();
  });

  state.eventSource = es;
}
