// SSE (Server-Sent Events) für Echtzeit-Updates.
// Verarbeitet granulare Events (task_created, task_updated, task_moved, task_deleted)
// und fällt auf Full-Refetch zurück bei project_update oder unbekannten Events.

import api from '../api';
import { state } from '../state';
import { renderBoard } from '../components/board';
import type { ProjectDoc, Task } from '../types';
import { notificationService } from './notification-service';
import { notificationHistoryService } from './notification-history-service';
import type { NotificationEntry, NotificationEventType } from './notification-history-service';

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
  // Offene Ansichten aktualisieren falls derselbe Task offen ist
  if (state.detailTask && state.detailTask.id === task.id) {
    Object.assign(state.detailTask, task);
  }
  if (state.editingTask && state.editingTask.id === task.id) {
    Object.assign(state.editingTask, task);
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

/**
 * Erstellt einen lokalen NotificationEntry aus SSE-Payload und fügt ihn
 * vorne in die Notification-History ein (ohne API-Call, da Backend
 * die Notification bereits persistiert hat).
 */
function prependHistoryEntry(
  eventType: NotificationEventType,
  data: Record<string, unknown>,
  projectId: string,
): void {
  const entry: NotificationEntry = {
    id: `local-${Date.now()}-${Math.random().toString(36).slice(2)}`,
    event_type: eventType,
    task_id: (data.id as string) || '',
    task_title: (data.title as string) || '?',
    project_id: projectId,
    actor: (data.actor as string | null) ?? null,
    read: false,
    created_at: new Date().toISOString(),
  }
  notificationHistoryService.prepend(entry)
}

export function subscribeSSE(projectId: string): void {
  if (state.eventSource) {
    state.eventSource.close();
    state.eventSource = null;
  }
  const es = new EventSource(`/api/projects/${projectId}/events`);

  // Granulare Events
  es.addEventListener('project_event', async (e: MessageEvent) => {
    let payload: SSEPayload;
    try {
      payload = JSON.parse(e.data);
    } catch {
      // Fallback: altes Format — Board-Update nur außerhalb eines Drags
      if (!state.isDragging) {
        state.project = await api.get<ProjectDoc>(`/api/projects/${projectId}`);
        renderBoard();
      }
      return;
    }

    // Notifications immer verarbeiten, Board-Updates nur wenn kein Drag läuft
    switch (payload.event) {
      case 'task_created': {
        const task = payload.data as unknown as Task;
        notificationService.notify(payload);
        prependHistoryEntry(payload.event as NotificationEventType, payload.data, projectId);
        if (!state.isDragging) {
          patchTask(task);
          // Glow für via SSE empfangene neue Tasks (von anderen Clients/Agenten)
          (window as any).__newTaskGlowId = task.id;
          renderBoard();
        }
        break;
      }
      case 'task_updated':
        notificationService.notify(payload);
        prependHistoryEntry(payload.event as NotificationEventType, payload.data, projectId);
        if (!state.isDragging) {
          patchTask(payload.data as unknown as Task);
          renderBoard();
        }
        break;

      case 'task_moved': {
        const actor = payload.data.actor as string | undefined;
        const me = state.currentUser?.display_name || state.currentUser?.username;
        // Eigene Aktion nicht ins Notification-Center — der Erfolgs-Toast reicht
        if (!me || actor !== me) {
          notificationService.notify(payload);
          prependHistoryEntry(payload.event as NotificationEventType, payload.data, projectId);
        }
        if (!state.isDragging) {
          patchTask(payload.data as unknown as Task);
          renderBoard();
        }
        break;
      }

      case 'task_commented':
        notificationService.notify(payload);
        prependHistoryEntry(payload.event as NotificationEventType, payload.data, projectId);
        break;

      case 'task_deleted':
        if (!state.isDragging) {
          removeTask((payload.data as { task_id: string }).task_id);
          renderBoard();
        }
        break;

      case 'project_update':
      default:
        // Full-Refetch für Projekt-Level-Änderungen (Spalten, User, etc.)
        if (!state.isDragging) {
          state.project = await api.get<ProjectDoc>(`/api/projects/${projectId}`);
          renderBoard();
        }
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

/**
 * Test-Hilfsfunktion: SSE-Event manuell simulieren.
 * Nur in Dev/Test-Umgebungen genutzt (Playwright-Tests).
 */
;(window as any).__simulateSSE = (event: string, data: Record<string, unknown>) => {
  notificationService.notify({ event, data });
};
