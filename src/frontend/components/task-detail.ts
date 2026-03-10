// Task-Detail – Bridge zu TaskDetail.vue.

import type { Task } from '../types';

export function openTaskDetail(task: Task): void {
  if (typeof window.__openTaskDetail === 'function') {
    window.__openTaskDetail(task);
  }
}

export function closeTaskDetail(): void {
  if (typeof window.__closeTaskDetail === 'function') {
    window.__closeTaskDetail();
  }
}
