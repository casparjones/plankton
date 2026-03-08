// Projekt- und Task-CRUD Operationen.

import api from '../api.js';
import { state } from '../state.js';
import { renderBoard } from '../components/board.js';
import { renderProjectList, updateProjectTitle } from '../components/sidebar.js';
import { subscribeSSE } from './sse-service.js';

function lastProjectKey() {
  const username = state.currentUser?.username || '';
  return `plankton_last_project_${username}`;
}

export function saveLastProject(id) {
  try { localStorage.setItem(lastProjectKey(), id); } catch {}
}

export function getLastProject() {
  try { return localStorage.getItem(lastProjectKey()); } catch { return null; }
}

export async function loadProjects() {
  state.projects = await api.get('/api/projects');
  renderProjectList();
}

export async function openProject(id) {
  state.project = await api.get(`/api/projects/${id}`);
  state.selectedTasks.clear();
  saveLastProject(id);
  renderProjectList();
  renderBoard();
  updateProjectTitle();
  subscribeSSE(id);
}

export async function createProject(title) {
  const payload = {
    _id: '',
    title: title || 'Untitled',
    columns: [
      { id: crypto.randomUUID(), title: 'Todo',        order: 0, color: '#90CAF9', hidden: false },
      { id: crypto.randomUUID(), title: 'In Progress', order: 1, color: '#FFCC80', hidden: false },
      { id: crypto.randomUUID(), title: 'Done',        order: 2, color: '#A5D6A7', hidden: false },
      { id: crypto.randomUUID(), title: '_archive',    order: 99, color: '#444',   hidden: true },
    ],
    users: [],
    tasks: [],
  };
  state.project = await api.post('/api/projects', payload);
  saveLastProject(state.project._id);
  await loadProjects();
  renderBoard();
  updateProjectTitle();
  subscribeSSE(state.project._id);
}

export async function renameProject(id, newTitle) {
  const project = await api.get(`/api/projects/${id}`);
  project.title = newTitle;
  await api.put(`/api/projects/${id}`, project);
  await loadProjects();
  if (state.project?._id === id) {
    state.project.title = newTitle;
    updateProjectTitle();
  }
}

export async function deleteProject(id) {
  const project = await api.get(`/api/projects/${id}?include_archived=true`);
  const rev = project._rev;
  await api.del(`/api/projects/${id}?rev=${rev}`);
  await loadProjects();
  if (state.project?._id === id) {
    if (state.projects.length > 0) {
      await openProject(state.projects[0]._id);
    } else {
      state.project = null;
      renderBoard();
      updateProjectTitle();
    }
  }
}

export async function saveTask(task) {
  state.project = await api.put(
      `/api/projects/${state.project._id}/tasks/${task.id}`,
      task
  );
  renderBoard();
}

export async function createTaskViaApi(task) {
  state.project = await api.post(`/api/projects/${state.project._id}/tasks`, task);
  renderBoard();
}

export async function deleteTask(taskId) {
  await api.del(`/api/projects/${state.project._id}/tasks/${taskId}`);
  state.project = await api.get(`/api/projects/${state.project._id}`);
  renderBoard();
}

export async function moveTask(taskId, columnId, order, skipRender) {
  try {
    state.project = await api.post(
        `/api/projects/${state.project._id}/tasks/${taskId}/move`,
        { column_id: columnId, order }
    );
    if (!skipRender) renderBoard();
  } catch (err) {
    console.error('Move failed:', err);
    renderBoard();
  }
}
