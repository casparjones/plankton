// Projekt-Menü (Dropdown, Editieren, JSON Import/Export, Prompt).

import api from '../api';
import { state } from '../state';
import { escapeHtml, columnName } from '../utils';
import { renderBoard } from './board';
import { renderJsonTree, toggleJsonView } from './json-view';
import { loadProjects, renameProject } from '../services/project-service';
import { updateProjectTitle } from './sidebar';
import { subscribeSSE } from '../services/sse-service';
import { openGitModal } from './git-settings';
import type { ProjectDoc } from '../types';

export function openProjectDropdown(): void {
  closeProjectDropdown();
  if (!state.project) return;

  const dropdown = document.getElementById('project-dropdown')!;
  dropdown.innerHTML = `
    <button class="proj-dropdown-item" data-action="edit">&#9998; Projekt editieren</button>
    <button class="proj-dropdown-item" data-action="git">&#128268; Git-Einstellungen</button>
    <button class="proj-dropdown-item" data-action="prompt">&#9733; Show Prompt</button>
  `;
  dropdown.classList.add('open');

  dropdown.addEventListener('click', (e: MouseEvent) => {
    const action = (e.target as HTMLElement).closest<HTMLElement>('[data-action]')?.dataset.action;
    closeProjectDropdown();
    if (action === 'edit') openProjectMenu();
    if (action === 'git') openGitModal();
    if (action === 'prompt') openPromptModal();
  });

  setTimeout(() => {
    document.addEventListener('click', closeProjectDropdown, { once: true });
  }, 0);
}

export function closeProjectDropdown(): void {
  const dropdown = document.getElementById('project-dropdown');
  if (dropdown) {
    dropdown.classList.remove('open');
    dropdown.innerHTML = '';
  }
}

export function openPromptModal(): void {
  if (!state.project) return;
  const prompt = generateProjectPrompt();
  document.getElementById('prompt-content')!.textContent = prompt;
  document.getElementById('prompt-modal')!.classList.add('open');
}

export function closePromptModal(): void {
  document.getElementById('prompt-modal')!.classList.remove('open');
}

function generateProjectPrompt(): string {
  const p = state.project!;
  const columns = (p.columns || []).filter(c => !c.hidden).sort((a, b) => a.order - b.order);
  const colList = columns.map(c => `  - id: "${c.id}", title: "${c.title}"`).join('\n');
  const existingTasks = (p.tasks || []).slice(0, 3);
  const taskExample = existingTasks.length > 0
    ? JSON.stringify(existingTasks[0], null, 2)
    : JSON.stringify({
        id: '',
        title: 'Beispiel-Task',
        description: 'Beschreibung des Tasks',
        column_id: columns[0]?.id || '',
        labels: ['feature'],
        order: 0,
        points: 5,
        worker: '',
        creator: '',
        comments: [],
        logs: [],
      }, null, 2);

  return `Du bist ein Projektmanagement-Assistent. Generiere Tasks als JSON für das Kanban-Board "${p.title}".

## Projekt-Struktur

Das Projekt hat folgende Spalten:
${colList}

## Task-Format

Jeder Task ist ein JSON-Objekt mit dieser Struktur:
${taskExample}

### Feld-Beschreibung:
- id: Leer lassen ("") – wird vom Server generiert
- title: Kurzer, prägnanter Titel des Tasks
- description: Ausführliche Beschreibung, Akzeptanzkriterien, Details
- column_id: ID der Spalte, in der der Task erscheinen soll (siehe Spalten oben)
- labels: Array von Strings, z.B. ["feature"], ["bug"], ["refactor"], ["docs"]
- order: Position innerhalb der Spalte (0 = oben)
- points: Story Points / Aufwand (0–100), z.B. 1=trivial, 3=klein, 5=mittel, 8=groß, 13=sehr groß
- worker: Name der zugewiesenen Person (leer lassen wenn unklar)
- creator: Name des Erstellers (leer lassen)
- comments: Array von Strings für Kommentare
- logs: Array von Strings für Logs (leer lassen)

## Antwort-Format

Antworte mit einem JSON-Array von Tasks:
[
  { "id": "", "title": "...", "description": "...", "column_id": "${columns[0]?.id || 'SPALTEN_ID'}", "labels": [...], "order": 0, "points": 5, "worker": "", "creator": "", "comments": [], "logs": [] },
  ...
]

## Aktuelle Tasks im Projekt (${(p.tasks || []).length} Stück):
${(p.tasks || []).length > 0 ? (p.tasks || []).map(t => `- [${columnName(t.column_id)}] ${t.title}`).join('\n') : '(keine)'}

Generiere jetzt Tasks basierend auf der folgenden Anforderung:
`;
}

export async function openProjectMenu(): Promise<void> {
  if (!state.project) return;
  const project = await api.get<ProjectDoc>(`/api/projects/${state.project._id}?include_archived=true`);
  (document.getElementById('proj-modal-title') as HTMLInputElement).value = project.title || '';
  (document.getElementById('proj-modal-json') as HTMLTextAreaElement).value = JSON.stringify(project, null, 2);
  renderJsonTree(project, document.getElementById('proj-json-tree')!);
  document.getElementById('proj-json-tree')!.style.display = '';
  (document.getElementById('proj-modal-json') as HTMLTextAreaElement).style.display = 'none';
  const toggleBtn = document.getElementById('proj-view-toggle')!;
  toggleBtn.textContent = 'Raw JSON';
  document.getElementById('project-modal')!.classList.add('open');
}

export function closeProjectMenu(): void {
  document.getElementById('project-modal')!.classList.remove('open');
}

export async function copyProjectJson(): Promise<void> {
  const textarea = document.getElementById('proj-modal-json') as HTMLTextAreaElement;
  try {
    await navigator.clipboard.writeText(textarea.value);
    const btn = document.getElementById('proj-modal-copy')!;
    btn.textContent = 'Kopiert!';
    setTimeout(() => { btn.textContent = 'In Zwischenablage kopieren'; }, 1500);
  } catch {
    textarea.select();
  }
}

export async function importProjectJson(): Promise<void> {
  const text = (document.getElementById('proj-modal-json') as HTMLTextAreaElement).value.trim();
  if (!text) return;
  let data: any;
  try {
    data = JSON.parse(text);
  } catch {
    alert('Ungültiges JSON');
    return;
  }
  if (!confirm('Neues Projekt aus diesem JSON erstellen?')) return;
  data._id = '';
  delete data._rev;
  data.title = data.title ? data.title + ' (Import)' : 'Import';
  state.project = await api.post<ProjectDoc>('/api/projects', data);
  await loadProjects();
  closeProjectMenu();
  renderBoard();
  updateProjectTitle();
  subscribeSSE(state.project!._id);
}

export async function saveProjectJson(): Promise<void> {
  if (!state.project) return;
  const textarea = document.getElementById('proj-modal-json') as HTMLTextAreaElement;
  const titleInput = document.getElementById('proj-modal-title') as HTMLInputElement;
  const text = textarea.value.trim();
  if (!text) return;

  let data: any;
  try {
    data = JSON.parse(text);
  } catch {
    alert('Ungültiges JSON');
    return;
  }

  const newTitle = titleInput.value.trim();
  if (newTitle) data.title = newTitle;

  data._id = state.project._id;
  data._rev = state.project._rev;

  if (!confirm('Projekt mit diesem JSON überschreiben?')) return;

  try {
    state.project = await api.put<ProjectDoc>(`/api/projects/${state.project._id}`, data);
    await loadProjects();
    closeProjectMenu();
    renderBoard();
    updateProjectTitle();
  } catch (err: any) {
    alert('Fehler beim Speichern: ' + err.message);
  }
}

export async function saveProjectTitle(): Promise<void> {
  if (!state.project) return;
  const titleInput = document.getElementById('proj-modal-title') as HTMLInputElement;
  const newTitle = titleInput.value.trim();
  if (newTitle && newTitle !== state.project.title) {
    await renameProject(state.project._id, newTitle);
  }
}
