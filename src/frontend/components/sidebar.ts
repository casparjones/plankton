// Sidebar: Projektliste gruppiert nach Owner, aufklappbar, mit Drag&Drop.

import Sortable from 'sortablejs';
import api from '../api';
import { state } from '../state';
import { openProject, deleteProject, loadProjects } from '../services/project-service';
import { toast, toastConfirm } from '../toast';
import type { ProjectDoc } from '../types';

const UNASSIGNED = '__unassigned__';

// Welche Gruppen sind manuell auf-/zugeklappt (überlebt Re-Render)
const manualToggle = new Set<string>();

// SortableJS-Instanzen für Cleanup
let sortableInstances: Sortable[] = [];

function groupProjects(): Map<string, ProjectDoc[]> {
  const groups = new Map<string, ProjectDoc[]>();
  for (const p of state.projects) {
    const key = p.owner || UNASSIGNED;
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key)!.push(p);
  }
  return groups;
}

function sortedGroupKeys(groups: Map<string, ProjectDoc[]>): string[] {
  const me = state.currentUser?.display_name || '';
  const keys = [...groups.keys()];
  return keys.sort((a, b) => {
    // Eigene Gruppe zuerst
    if (a === me) return -1;
    if (b === me) return 1;
    // Unassigned zuletzt
    if (a === UNASSIGNED) return 1;
    if (b === UNASSIGNED) return -1;
    return a.localeCompare(b);
  });
}

function isExpanded(key: string): boolean {
  if (manualToggle.has(key)) {
    // Toggle-State umkehren gegenüber Default
    const isMe = key === (state.currentUser?.display_name || '');
    return !isMe; // Wenn manuell getoggled: invertiert
  }
  // Default: eigene Gruppe offen, Rest zu
  return key === (state.currentUser?.display_name || '');
}

function toggleGroup(key: string): void {
  if (manualToggle.has(key)) {
    manualToggle.delete(key);
  } else {
    manualToggle.add(key);
  }
  renderProjectList();
}

async function reassignOwner(projectId: string, newOwner: string | null): Promise<void> {
  try {
    const project = await api.get<ProjectDoc>(`/api/projects/${projectId}`);
    project.owner = newOwner;
    await api.put(`/api/projects/${projectId}`, project);
    await loadProjects();
    toast.success(`"${project.title}" → ${newOwner ?? 'Nicht zugeordnet'}`);
  } catch (err) {
    console.error('Reassign failed:', err);
    toast.error('Zuweisen fehlgeschlagen');
    await loadProjects();
  }
}

function createProjectItem(p: ProjectDoc): HTMLLIElement {
  const li = document.createElement('li');
  li.className = 'project-item' + (p._id === state.project?._id ? ' active' : '');
  li.dataset.id = p._id;

  const nameSpan = document.createElement('span');
  nameSpan.className = 'project-name';
  nameSpan.textContent = p.title;
  nameSpan.addEventListener('click', () => openProject(p.slug || p._id));
  li.appendChild(nameSpan);

  if (state.projects.length > 1) {
    const delBtn = document.createElement('button');
    delBtn.className = 'project-delete-btn';
    delBtn.textContent = '×';
    delBtn.title = 'Projekt löschen';
    delBtn.addEventListener('click', (e: Event) => {
      e.stopPropagation();
      toastConfirm(`Projekt "${p.title}" löschen?`).then(ok => ok && deleteProject(p._id));
    });
    li.appendChild(delBtn);
  }

  return li;
}

export function renderProjectList(): void {
  const list = document.getElementById('project-list');
  if (!list) return;
  list.innerHTML = '';

  // Cleanup alte SortableJS-Instanzen
  for (const s of sortableInstances) s.destroy();
  sortableInstances = [];

  const groups = groupProjects();
  const keys = sortedGroupKeys(groups);

  // Alle bekannten User als mögliche Gruppen anbieten (auch ohne Projekte)
  for (const u of state.allUsers) {
    if (!groups.has(u.display_name)) {
      groups.set(u.display_name, []);
      keys.push(u.display_name);
    }
  }
  // Keys neu sortieren nach Hinzufügen
  keys.sort((a, b) => {
    const me = state.currentUser?.display_name || '';
    if (a === me) return -1;
    if (b === me) return 1;
    if (a === UNASSIGNED) return 1;
    if (b === UNASSIGNED) return -1;
    return a.localeCompare(b);
  });

  // Wenn nur eine Gruppe oder gar keine User → flache Liste
  if (keys.length <= 1 && state.allUsers.length <= 1) {
    const projects = groups.get(keys[0] || UNASSIGNED) || state.projects;
    for (const p of projects) {
      list.appendChild(createProjectItem(p));
    }
    return;
  }

  for (const key of keys) {
    const projects = groups.get(key) || [];
    const expanded = isExpanded(key);
    const displayName = key === UNASSIGNED ? 'Nicht zugeordnet' : key;

    const groupEl = document.createElement('li');
    groupEl.className = 'owner-group' + (expanded ? ' owner-group--expanded' : '');
    groupEl.dataset.owner = key;

    // Header
    const header = document.createElement('div');
    header.className = 'owner-group-header';
    header.addEventListener('click', () => toggleGroup(key));

    const toggle = document.createElement('span');
    toggle.className = 'owner-toggle';
    toggle.textContent = expanded ? '▼' : '▶';
    header.appendChild(toggle);

    const name = document.createElement('span');
    name.className = 'owner-name';
    name.textContent = displayName;
    header.appendChild(name);

    const count = document.createElement('span');
    count.className = 'owner-count';
    count.textContent = String(projects.length);
    header.appendChild(count);

    groupEl.appendChild(header);

    // Project-Liste (aufklappbar)
    const projectsUl = document.createElement('ul');
    projectsUl.className = 'owner-projects';
    projectsUl.dataset.owner = key;

    for (const p of projects) {
      projectsUl.appendChild(createProjectItem(p));
    }

    groupEl.appendChild(projectsUl);
    list.appendChild(groupEl);

    // SortableJS für Drag&Drop zwischen Gruppen
    const sortable = Sortable.create(projectsUl, {
      group: 'sidebar-projects',
      animation: 150,
      delay: 400,
      delayOnTouchOnly: true,
      touchStartThreshold: 5,
      draggable: '.project-item',
      ghostClass: 'sidebar-drag-ghost',
      onAdd(evt) {
        const projectId = (evt.item as HTMLElement).dataset.id;
        const targetOwner = (evt.to as HTMLElement).dataset.owner;
        if (projectId && targetOwner) {
          reassignOwner(projectId, targetOwner === UNASSIGNED ? null : targetOwner);
        }
      },
    });
    sortableInstances.push(sortable);
  }
}

export function updateProjectTitle(): void {
  const el = document.getElementById('project-title');
  if (el) el.textContent = state.project?.title || '';
}
