// Task-Modal – Bridge zu TaskModal.vue.
// Legacy-Module importieren weiterhin diese Funktionen.

import { state } from '../state';
import type { Task } from '../types';

export function tasksForColumn(columnId: string): Task[] {
  return (state.project?.tasks || [])
      .filter((t: Task) => t.column_id === columnId)
      .sort((a: Task, b: Task) => a.order - b.order);
}

export function openNewTaskModal(columnId: string): void {
  if (typeof window.__openNewTaskModal === 'function') {
    window.__openNewTaskModal(columnId);
  }
}

export function openTaskModal(task: Task, isNew: boolean): void {
  if (isNew) {
    if (typeof window.__openNewTaskModal === 'function') {
      window.__openNewTaskModal(task.column_id);
    }
  } else {
    if (typeof window.__openTaskModal === 'function') {
      window.__openTaskModal(task);
    }
  }
}

export function closeTaskModal(): void {
  if (typeof window.__closeTaskModal === 'function') {
    window.__closeTaskModal();
  }
}

export function renderModalComments(): void {
  // Nicht mehr nötig – Vue rendert Kommentare reaktiv.
}

// taskToItem() wird nicht mehr benötigt da KanbanBoard.vue Tasks direkt rendert.
// Trotzdem exportieren für Kompatibilität falls irgendwo referenziert.
export function taskToItem(task: Task): { id: string; title: string } {
  return { id: task.id, title: task.title };
}
