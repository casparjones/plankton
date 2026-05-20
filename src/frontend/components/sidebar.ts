// Sidebar: Projektliste gruppiert nach Owner, aufklappbar, mit Drag&Drop.
// Enthält Suche (Frontend-Filter) und Sortierung (persistiert in localStorage).

import Sortable from 'sortablejs';
import api from '../api';
import { state } from '../state';
import { openProject, deleteProject, loadProjects, createProject } from '../services/project-service';
import { toastPrompt } from '../toast';
import { toast, toastConfirm } from '../toast';
import { t } from '../i18n';
import type { ProjectDoc } from '../types';

const UNASSIGNED = '__unassigned__';

// ─── Suche & Sortierung ─────────────────────────────────────────────────────

export type SortMode = 'custom' | 'alpha-asc' | 'alpha-desc' | 'updated-desc' | 'updated-asc' | 'task-count';

const SORT_STORAGE_KEY = 'plankton_sidebar_sort';

let currentSearch = '';
let currentSort: SortMode = (localStorage.getItem(SORT_STORAGE_KEY) as SortMode) || 'custom';
let sortDropdownOpen = false;

function getProjectLastUpdated(p: ProjectDoc): number {
  if (!p.tasks || p.tasks.length === 0) return 0;
  return Math.max(...p.tasks.map(task => {
    const d = task.updated_at ? new Date(task.updated_at).getTime() : 0;
    return isNaN(d) ? 0 : d;
  }));
}

function sortProjects(projects: ProjectDoc[]): ProjectDoc[] {
  if (currentSort === 'custom') return projects;
  const sorted = [...projects];
  switch (currentSort) {
    case 'alpha-asc':
      sorted.sort((a, b) => a.title.localeCompare(b.title));
      break;
    case 'alpha-desc':
      sorted.sort((a, b) => b.title.localeCompare(a.title));
      break;
    case 'updated-desc':
      sorted.sort((a, b) => getProjectLastUpdated(b) - getProjectLastUpdated(a));
      break;
    case 'updated-asc':
      sorted.sort((a, b) => getProjectLastUpdated(a) - getProjectLastUpdated(b));
      break;
    case 'task-count':
      sorted.sort((a, b) => (b.tasks?.length ?? 0) - (a.tasks?.length ?? 0));
      break;
  }
  return sorted;
}

function filterProjects(projects: ProjectDoc[]): ProjectDoc[] {
  if (!currentSearch.trim()) return projects;
  const q = currentSearch.trim().toLowerCase();
  return projects.filter(p =>
    p.title.toLowerCase().includes(q) ||
    (p.slug && p.slug.toLowerCase().includes(q))
  );
}

function sortModeLabel(mode: SortMode): string {
  switch (mode) {
    case 'custom':       return t('sidebar.sortCustom');
    case 'alpha-asc':    return t('sidebar.sortAlphaAsc');
    case 'alpha-desc':   return t('sidebar.sortAlphaDesc');
    case 'updated-desc': return t('sidebar.sortUpdatedDesc');
    case 'updated-asc':  return t('sidebar.sortUpdatedAsc');
    case 'task-count':   return t('sidebar.sortTaskCount');
  }
}

function setSort(mode: SortMode): void {
  currentSort = mode;
  localStorage.setItem(SORT_STORAGE_KEY, mode);
  sortDropdownOpen = false;
  renderProjectList();
}

let searchVisible = false;

function createSidebarHeader(): HTMLDivElement {
  const container = document.createElement('div');
  container.className = 'sidebar-search-sort px-3 pt-2 pb-1.5 border-b border-border sticky top-0 bg-surface z-10';
  container.id = 'sidebar-search-sort';

  // Icon-Button-Zeile: + Projekt links, Search+Sort rechts
  const iconRow = document.createElement('div');
  iconRow.className = 'flex items-center justify-between mb-1';

  // + Projekt Button (links)
  const newProjectBtn = document.createElement('button');
  newProjectBtn.id = 'new-project-btn';
  newProjectBtn.className = 'bg-transparent border border-border rounded-md text-text-dim cursor-pointer font-sans text-[11px] px-2 h-7 flex items-center gap-1 leading-none transition-all hover:border-accent hover:text-accent';
  newProjectBtn.textContent = t('sidebar.newProject') || '+ Projekt';
  newProjectBtn.addEventListener('click', async () => {
    const name = await toastPrompt(t('sidebar.newProject') || '+ Projekt', t('sidebar.projectPlaceholder') || 'Projektname…');
    if (name) createProject(name);
  });
  iconRow.appendChild(newProjectBtn);

  // Rechte Icon-Gruppe: Search + Sort
  const rightBtns = document.createElement('div');
  rightBtns.className = 'flex items-center gap-1';

  // Such-Button
  const searchToggleBtn = document.createElement('button');
  searchToggleBtn.className = 'bg-transparent border border-border rounded-md text-text-dim cursor-pointer text-sm w-7 h-7 flex items-center justify-center leading-none transition-all hover:border-accent hover:text-accent';
  searchToggleBtn.innerHTML = '&#128269;';
  searchToggleBtn.title = t('sidebar.searchPlaceholder') || 'Suche';
  searchToggleBtn.addEventListener('click', (e: Event) => {
    e.stopPropagation();
    searchVisible = !searchVisible;
    const wrap = document.getElementById('sidebar-search-wrap');
    if (wrap) {
      wrap.style.display = searchVisible ? 'block' : 'none';
    }
    if (searchVisible) {
      const input = document.getElementById('sidebar-search-input') as HTMLInputElement | null;
      if (input) { input.focus(); }
    } else {
      // Suche schließen = Suchtext löschen
      currentSearch = '';
      const input = document.getElementById('sidebar-search-input') as HTMLInputElement | null;
      if (input) input.value = '';
      renderProjectList();
    }
  });
  rightBtns.appendChild(searchToggleBtn);

  // Sort-Button (Icon)
  const sortRow = document.createElement('div');
  sortRow.className = 'relative';

  const sortToggleBtn = document.createElement('button');
  sortToggleBtn.setAttribute('data-sort-toggle', '');
  sortToggleBtn.className = 'bg-transparent border border-border rounded-md text-text-dim cursor-pointer text-sm w-7 h-7 flex items-center justify-center leading-none transition-all hover:border-accent hover:text-accent';
  sortToggleBtn.innerHTML = '&#8597;';
  sortToggleBtn.title = t('sidebar.sortLabel') || 'Sortierung';

  sortToggleBtn.addEventListener('click', (e: Event) => {
    e.stopPropagation();
    sortDropdownOpen = !sortDropdownOpen;
    const dropdown = document.getElementById('sidebar-sort-dropdown');
    if (dropdown) {
      dropdown.style.display = sortDropdownOpen ? 'block' : 'none';
    }
  });
  sortRow.appendChild(sortToggleBtn);

  // Sort-Dropdown
  const sortDropdown = document.createElement('div');
  sortDropdown.id = 'sidebar-sort-dropdown';
  sortDropdown.className = 'absolute left-0 top-full mt-0.5 bg-surface border border-border rounded-md shadow-[0_4px_16px_rgba(0,0,0,0.3)] z-50 py-0.5 min-w-[160px]';
  sortDropdown.style.display = 'none';

  const sortModes: SortMode[] = ['custom', 'alpha-asc', 'alpha-desc', 'updated-desc', 'updated-asc', 'task-count'];
  for (const mode of sortModes) {
    const item = document.createElement('button');
    item.setAttribute('data-sort-option', mode);
    const isActive = mode === currentSort;
    if (isActive) item.setAttribute('data-sort-active', 'true');
    item.className = `sort-option w-full text-left px-3 py-1.5 text-[12px] font-sans cursor-pointer bg-transparent border-none transition-colors hover:bg-surface-2 hover:text-accent ${isActive ? 'sort-active text-accent' : 'text-text'}`;
    item.setAttribute('aria-pressed', isActive ? 'true' : 'false');
    item.textContent = sortModeLabel(mode);
    item.addEventListener('click', (e: Event) => {
      e.stopPropagation();
      setSort(mode);
    });
    sortDropdown.appendChild(item);
  }
  sortRow.appendChild(sortDropdown);
  rightBtns.appendChild(sortRow);
  iconRow.appendChild(rightBtns);
  container.appendChild(iconRow);

  // Suchfeld (standardmäßig verborgen)
  const searchWrap = document.createElement('div');
  searchWrap.id = 'sidebar-search-wrap';
  searchWrap.className = 'relative mb-1';
  searchWrap.style.display = searchVisible ? 'block' : 'none';

  const searchInput = document.createElement('input');
  searchInput.type = 'text';
  searchInput.id = 'sidebar-search-input';
  searchInput.placeholder = t('sidebar.searchPlaceholder');
  searchInput.value = currentSearch;
  searchInput.autocomplete = 'off';
  searchInput.className = 'w-full bg-surface-2 border border-border rounded-md text-text font-sans text-[12px] px-2.5 py-1.5 pr-7 outline-none transition-colors focus:border-accent placeholder:text-text-dim';
  searchInput.addEventListener('input', () => {
    currentSearch = searchInput.value;
    renderProjectList();
  });
  searchInput.addEventListener('keydown', (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      currentSearch = '';
      searchInput.value = '';
      searchVisible = false;
      const wrap = document.getElementById('sidebar-search-wrap');
      if (wrap) wrap.style.display = 'none';
      renderProjectList();
    }
  });
  searchWrap.appendChild(searchInput);

  // Clear-Button im Suchfeld
  const clearBtn = document.createElement('button');
  clearBtn.className = 'absolute right-1.5 top-1/2 -translate-y-1/2 text-text-dim text-xs bg-transparent border-none cursor-pointer hover:text-text leading-none p-0.5';
  clearBtn.textContent = '×';
  clearBtn.title = 'Suche zurücksetzen';
  clearBtn.style.display = currentSearch ? 'block' : 'none';
  clearBtn.addEventListener('click', () => {
    currentSearch = '';
    searchInput.value = '';
    renderProjectList();
  });
  searchWrap.appendChild(clearBtn);
  container.appendChild(searchWrap);

  // Dropdown schließen bei Klick außerhalb
  document.addEventListener('click', () => {
    if (sortDropdownOpen) {
      sortDropdownOpen = false;
      const dropdown = document.getElementById('sidebar-sort-dropdown');
      if (dropdown) dropdown.style.display = 'none';
    }
  }, { once: false, capture: false });

  return container;
}

/** Initialisiert den Sidebar-Header (Suche + Sort) – wird einmalig aufgerufen. */
export function initSidebarHeader(): void {
  const list = document.getElementById('project-list');
  if (!list) return;
  // Header nur einmalig einfügen
  if (document.getElementById('sidebar-search-sort')) return;
  const header = createSidebarHeader();
  list.before(header);
}

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

  // Sidebar-Header (Suche + Sort) sicherstellen
  initSidebarHeader();

  // Suche + Sort auf die globale Projektliste anwenden
  const visibleProjects = sortProjects(filterProjects(state.projects));

  // Clear-Button-Sichtbarkeit aktualisieren
  const clearBtn = document.querySelector('#sidebar-search-wrap .absolute') as HTMLElement | null;
  if (clearBtn) clearBtn.style.display = currentSearch ? 'block' : 'none';

  // Drag & Drop nur bei Custom-Sortierung
  const dragEnabled = currentSort === 'custom';

  // Cleanup alte SortableJS-Instanzen
  for (const s of sortableInstances) s.destroy();
  sortableInstances = [];

  // Gruppen auf Basis der gefilterten/sortierten Projekte aufbauen
  const groupsFiltered = new Map<string, ProjectDoc[]>();
  for (const p of visibleProjects) {
    const key = p.owner || UNASSIGNED;
    if (!groupsFiltered.has(key)) groupsFiltered.set(key, []);
    groupsFiltered.get(key)!.push(p);
  }

  const groups = groupsFiltered;
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
    const projects = groups.get(keys[0] || UNASSIGNED) || visibleProjects;
    for (const p of projects) {
      list.appendChild(createProjectItem(p));
    }
    // SortableJS für flache Liste (nur bei Custom-Sortierung)
    if (dragEnabled) {
      const flatSortable = Sortable.create(list as HTMLElement, {
        group: 'sidebar-projects',
        animation: 150,
        delay: 400,
        delayOnTouchOnly: true,
        touchStartThreshold: 5,
        draggable: '.project-item',
        ghostClass: 'sidebar-drag-ghost',
        async onEnd() {
          const allItems = list.querySelectorAll<HTMLElement>('.project-item[data-id]');
          const ids = Array.from(allItems).map(el => el.dataset.id!).filter(Boolean);
          if (ids.length === 0) return;
          const idIndex = new Map(ids.map((id, i) => [id, i]));
          state.projects.sort((a, b) => {
            const ai = idIndex.get(a._id) ?? Number.MAX_SAFE_INTEGER;
            const bi = idIndex.get(b._id) ?? Number.MAX_SAFE_INTEGER;
            return ai - bi;
          });
          try {
            await api.post('/api/projects/reorder', { ids });
            toast.success(t('sidebar.reorderSaved') || 'Reihenfolge gespeichert', { timeout: 1500 });
          } catch (err) {
            console.error('Reorder projects failed:', err);
            toast.error('Reihenfolge konnte nicht gespeichert werden');
          }
        },
      });
      sortableInstances.push(flatSortable);
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

    // SortableJS für Drag&Drop zwischen Gruppen (nur bei Custom-Sortierung)
    const sortable = Sortable.create(projectsUl, {
      group: dragEnabled ? 'sidebar-projects' : { name: 'sidebar-projects-disabled', pull: false, put: false },
      animation: 150,
      delay: 400,
      delayOnTouchOnly: true,
      touchStartThreshold: 5,
      draggable: dragEnabled ? '.project-item' : '.project-item-never',
      ghostClass: 'sidebar-drag-ghost',
      onAdd(evt) {
        if (!dragEnabled) return;
        const projectId = (evt.item as HTMLElement).dataset.id;
        const targetOwner = (evt.to as HTMLElement).dataset.owner;
        if (projectId && targetOwner) {
          reassignOwner(projectId, targetOwner === UNASSIGNED ? null : targetOwner);
        }
      },
      async onEnd() {
        if (!dragEnabled) return;
        // Alle sichtbaren Projekt-Items in ihrer neuen DOM-Reihenfolge auslesen
        const allItems = document.querySelectorAll<HTMLElement>('#project-list .project-item[data-id]');
        const ids = Array.from(allItems).map(el => el.dataset.id!).filter(Boolean);
        if (ids.length === 0) return;
        // Optimistisch: state.projects entsprechend umsortieren
        const idIndex = new Map(ids.map((id, i) => [id, i]));
        state.projects.sort((a, b) => {
          const ai = idIndex.get(a._id) ?? Number.MAX_SAFE_INTEGER;
          const bi = idIndex.get(b._id) ?? Number.MAX_SAFE_INTEGER;
          return ai - bi;
        });
        // Neue Reihenfolge ans Backend senden
        try {
          await api.post('/api/projects/reorder', { ids });
          toast.success(t('sidebar.reorderSaved') || 'Reihenfolge gespeichert', { timeout: 1500 });
        } catch (err) {
          console.error('Reorder projects failed:', err);
          toast.error('Reihenfolge konnte nicht gespeichert werden');
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
