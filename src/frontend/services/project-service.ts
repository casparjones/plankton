// Projekt- und Task-CRUD Operationen.

import api from '../api';
import { state } from '../state';
import { renderBoard } from '../components/board';
import { renderProjectList, updateProjectTitle } from '../components/sidebar';
import { subscribeSSE } from './sse-service';
import type { ProjectDoc, Task } from '../types';

function lastProjectKey(): string {
  const username = state.currentUser?.username || '';
  return `plankton_last_project_${username}`;
}

export function saveLastProject(id: string): void {
  try { localStorage.setItem(lastProjectKey(), id); } catch {}
}

export function getLastProject(): string | null {
  try { return localStorage.getItem(lastProjectKey()); } catch { return null; }
}

export async function loadProjects(): Promise<void> {
  state.projects = await api.get<ProjectDoc[]>('/api/projects');
  renderProjectList();
}

export async function openProject(id: string, skipPush?: boolean): Promise<void> {
  state.project = await api.get<ProjectDoc>(`/api/projects/${id}`);
  state.selectedTasks.clear();
  const slug = state.project.slug || state.project._id;
  saveLastProject(slug);
  renderProjectList();
  renderBoard();
  updateProjectTitle();
  subscribeSSE(state.project._id);
  if (!skipPush) {
    history.pushState({ project: slug }, '', `/p/${slug}`);
  }
}

export async function createProject(title: string): Promise<void> {
  const payload: ProjectDoc = {
    _id: '',
    slug: '',
    title: title || 'Untitled',
    columns: [
      { id: crypto.randomUUID(), title: 'Todo',        order: 0, color: '#90CAF9', hidden: false, slug: '', locked: false },
      { id: crypto.randomUUID(), title: 'In Progress', order: 1, color: '#FFCC80', hidden: false, slug: '', locked: false },
      { id: crypto.randomUUID(), title: 'Testing',     order: 2, color: '#CE93D8', hidden: false, slug: '', locked: false },
      { id: crypto.randomUUID(), title: 'Done',        order: 3, color: '#A5D6A7', hidden: false, slug: '', locked: false },
      { id: crypto.randomUUID(), title: '_archive',    order: 99, color: '#444',   hidden: true,  slug: '', locked: false },
    ],
    users: [],
    tasks: [],
  };
  state.project = await api.post<ProjectDoc>('/api/projects', payload);
  const newSlug = state.project.slug || state.project._id;
  saveLastProject(newSlug);
  await loadProjects();
  renderBoard();
  updateProjectTitle();
  subscribeSSE(state.project._id);
  history.pushState({ project: newSlug }, '', `/p/${newSlug}`);
}

export async function renameProject(id: string, newTitle: string): Promise<void> {
  const project = await api.get<ProjectDoc>(`/api/projects/${id}`);
  project.title = newTitle;
  const updated = await api.put<ProjectDoc>(`/api/projects/${id}`, project);
  await loadProjects();
  if (state.project?._id === id || state.project?.slug === id) {
    state.project = updated;
    updateProjectTitle();
    const newSlug = updated.slug || updated._id;
    saveLastProject(newSlug);
    history.replaceState({ project: newSlug }, '', `/p/${newSlug}`);
  }
}

export async function deleteProject(id: string): Promise<void> {
  const project = await api.get<ProjectDoc>(`/api/projects/${id}?include_archived=true`);
  const rev = project._rev;
  await api.del(`/api/projects/${id}?rev=${rev}`);
  await loadProjects();
  if (state.project?._id === id || state.project?.slug === id) {
    if (state.projects.length > 0) {
      await openProject(state.projects[0]._id);
    } else {
      state.project = null;
      renderBoard();
      updateProjectTitle();
    }
  }
}

export async function saveTask(task: Task): Promise<void> {
  state.project = await api.put<ProjectDoc>(
      `/api/projects/${state.project!._id}/tasks/${task.id}`,
      task
  );
  renderBoard();
}

export async function createTaskViaApi(task: Task): Promise<void> {
  state.project = await api.post<ProjectDoc>(`/api/projects/${state.project!._id}/tasks`, task);
  renderBoard();
}

export async function deleteTask(taskId: string): Promise<void> {
  await api.del(`/api/projects/${state.project!._id}/tasks/${taskId}`);
  state.project = await api.get<ProjectDoc>(`/api/projects/${state.project!._id}`);
  renderBoard();
}

export async function moveTask(taskId: string, columnId: string, order: number, skipRender?: boolean): Promise<void> {
  try {
    state.project = await api.post<ProjectDoc>(
        `/api/projects/${state.project!._id}/tasks/${taskId}/move`,
        { column_id: columnId, order }
    );
    if (!skipRender) renderBoard();
  } catch (err) {
    console.error('Move failed:', err);
    renderBoard();
  }
}
