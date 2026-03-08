// Sidebar: Projektliste und Titel.

import { state } from '../state.js';
import { openProject, deleteProject } from '../services/project-service.js';

export function renderProjectList() {
  const list = document.getElementById('project-list');
  list.innerHTML = '';
  state.projects.forEach(p => {
    const li = document.createElement('li');
    li.className = 'project-item' + (p._id === state.project?._id ? ' active' : '');
    li.dataset.id = p._id;

    const nameSpan = document.createElement('span');
    nameSpan.className = 'project-name';
    nameSpan.textContent = p.title;
    nameSpan.addEventListener('click', () => openProject(p._id));

    li.appendChild(nameSpan);

    if (state.projects.length > 1) {
      const delBtn = document.createElement('button');
      delBtn.className = 'project-delete-btn';
      delBtn.textContent = '×';
      delBtn.title = 'Projekt löschen';
      delBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        if (confirm(`Projekt "${p.title}" und alle Tasks wirklich löschen?`)) deleteProject(p._id);
      });
      li.appendChild(delBtn);
    }

    list.appendChild(li);
  });
}

export function updateProjectTitle() {
  const el = document.getElementById('project-title');
  if (el) el.textContent = state.project?.title || '';
}
