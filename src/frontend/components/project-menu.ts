// Projekt-Menü (Dropdown, Editieren, JSON Import/Export, Prompt mit Tabs).

import api from '../api';
import { state } from '../state';
import { escapeHtml, columnName } from '../utils';
import { renderBoard } from './board';
import { renderJsonTree, toggleJsonView } from './json-view';
import { loadProjects, renameProject } from '../services/project-service';
import { updateProjectTitle } from './sidebar';
import { subscribeSSE } from '../services/sse-service';
import { toastConfirm } from '../toast';
import { openGitModal } from './git-settings';
import { generateSecretsMd, generateRulesMd, generateWorkflowMd } from './prompt-generator';
import type { ProjectDoc, AgentToken } from '../types';

// Zwischenspeicher für geladene Tokens.
let cachedTokens: AgentToken[] = [];
// Aktuell sichtbarer Output-Tab.
let activeOutputTab = 'secrets';

export function openProjectDropdown(): void {
  closeProjectDropdown();
  if (!state.project) return;

  const dropdown = document.getElementById('project-dropdown')!;
  dropdown.innerHTML = `
    <button class="proj-dropdown-item" data-action="edit">&#9998; Projekt editieren</button>
    <button class="proj-dropdown-item" data-action="prompt">&#9733; Show Prompt</button>
    <button class="proj-dropdown-item" data-action="cli">&#9881; Install CLI</button>
  `;
  dropdown.classList.add('open');

  dropdown.addEventListener('click', (e: MouseEvent) => {
    const action = (e.target as HTMLElement).closest<HTMLElement>('[data-action]')?.dataset.action;
    closeProjectDropdown();
    if (action === 'edit') openProjectMenu();
    if (action === 'git') openGitModal();
    if (action === 'prompt') openPromptModal();
    if (action === 'cli') openCliModal();
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
  // Simple-Tab: Prompt generieren.
  const prompt = generateProjectPrompt();
  document.getElementById('prompt-content')!.textContent = prompt;
  // Plankton-Tab: URL vorbelegen.
  const urlInput = document.getElementById('prompt-plankton-url') as HTMLInputElement;
  if (urlInput && !urlInput.value) {
    urlInput.value = window.location.origin;
  }
  document.getElementById('prompt-modal')!.classList.add('open');
  // Tokens laden wenn Plankton-Tab aktiv ist oder beim ersten Öffnen.
  loadTokensForPrompt();
}

export function closePromptModal(): void {
  document.getElementById('prompt-modal')!.classList.remove('open');
}

export function openCliModal(): void {
  const url = window.location.origin;
  // Install-Befehl setzen.
  const installCmd = document.getElementById('cli-install-cmd');
  if (installCmd) installCmd.textContent = `curl -fsSL ${url}/install | bash`;
  const loginCmd = document.getElementById('cli-login-cmd');
  if (loginCmd) loginCmd.textContent = `plankton login ${url}`;
  const updateCmd = document.getElementById('cli-update-cmd');
  if (updateCmd) updateCmd.textContent = `curl -fsSL ${url}/install | bash`;
  document.getElementById('cli-modal')!.classList.add('open');
}

export function closeCliModal(): void {
  document.getElementById('cli-modal')!.classList.remove('open');
}

export function initCliModal(): void {
  document.getElementById('cli-modal-close')?.addEventListener('click', closeCliModal);
  document.getElementById('cli-modal')?.addEventListener('click', (e: Event) => {
    if ((e.target as HTMLElement).id === 'cli-modal') closeCliModal();
  });
  // Copy-Buttons.
  document.querySelectorAll('[data-cli-copy]').forEach(btn => {
    btn.addEventListener('click', async () => {
      const targetId = (btn as HTMLElement).dataset.cliCopy!;
      const el = document.getElementById(targetId);
      if (el) await copyToClipboard(el.textContent || '', btn as HTMLElement);
    });
  });
}

/** Registriert alle Event-Listener für das Prompt-Modal (Tabs + Aktionen). */
export function initPromptTabs(): void {
  // Modal schließen.
  document.getElementById('prompt-modal-close')?.addEventListener('click', closePromptModal);
  document.getElementById('prompt-modal')?.addEventListener('click', (e: Event) => {
    if ((e.target as HTMLElement).id === 'prompt-modal') closePromptModal();
  });

  // Simple-Tab: Copy-Button.
  document.getElementById('prompt-copy-btn')?.addEventListener('click', async () => {
    const text = document.getElementById('prompt-content')?.textContent || '';
    await copyToClipboard(text, document.getElementById('prompt-copy-btn')!);
  });

  // Haupt-Tabs (Simple / Plankton).
  document.querySelectorAll('.prompt-tab').forEach(tab => {
    tab.addEventListener('click', () => {
      const tabName = (tab as HTMLElement).dataset.promptTab;
      // Tabs umschalten.
      document.querySelectorAll('.prompt-tab').forEach(t => t.classList.remove('prompt-tab-active'));
      tab.classList.add('prompt-tab-active');
      // Content umschalten.
      document.querySelectorAll('.prompt-tab-content').forEach(c => c.classList.remove('prompt-tab-visible'));
      document.getElementById(`prompt-tab-${tabName}`)?.classList.add('prompt-tab-visible');
      // Tokens laden wenn Plankton-Tab.
      if (tabName === 'plankton') loadTokensForPrompt();
    });
  });

  // Generieren-Button.
  document.getElementById('prompt-generate-btn')?.addEventListener('click', generateFiles);

  // Output-Tabs (secrets / rules / workflow).
  document.querySelectorAll('.prompt-output-tab').forEach(tab => {
    tab.addEventListener('click', () => {
      activeOutputTab = (tab as HTMLElement).dataset.outputTab || 'secrets';
      document.querySelectorAll('.prompt-output-tab').forEach(t => t.classList.remove('prompt-output-tab-active'));
      tab.classList.add('prompt-output-tab-active');
      document.querySelectorAll('.prompt-output-content').forEach(c => c.classList.remove('prompt-tab-visible'));
      document.getElementById(`prompt-out-${activeOutputTab}`)?.classList.add('prompt-tab-visible');
    });
  });

  // Output: Copy + Download.
  document.getElementById('prompt-out-copy')?.addEventListener('click', async () => {
    const pre = document.getElementById(`prompt-out-${activeOutputTab}-pre`);
    if (pre) await copyToClipboard(pre.textContent || '', document.getElementById('prompt-out-copy')!);
  });
  document.getElementById('prompt-out-download')?.addEventListener('click', () => {
    const pre = document.getElementById(`prompt-out-${activeOutputTab}-pre`);
    if (pre) downloadFile(`${activeOutputTab}.md`, pre.textContent || '');
  });
}

/** Lädt Tokens vom Server. Erstellt automatisch drei Rollen-Tokens wenn keine vorhanden. */
async function loadTokensForPrompt(): Promise<void> {
  const list = document.getElementById('prompt-token-list')!;
  const loading = document.getElementById('prompt-token-loading')!;
  loading.style.display = '';

  try {
    let tokens: AgentToken[] = await api.get<AgentToken[]>('/api/admin/tokens');

    // Automatisch drei Rollen-Tokens anlegen wenn keine existieren.
    if (tokens.length === 0) {
      const roles = [
        { name: 'Architect', role: 'manager' },
        { name: 'Developer', role: 'developer' },
        { name: 'Tester', role: 'tester' },
      ];
      for (const r of roles) {
        const created = await api.post<AgentToken>('/api/admin/tokens', r);
        tokens.push(created);
      }
    }

    cachedTokens = tokens;
    renderTokenList(list, tokens);
  } catch {
    // Kein Admin-Zugriff – Tokens können nicht geladen werden.
    list.innerHTML = '<p class="prompt-token-hint">Nur Admins können Tokens verwalten.</p>';
  }

  loading.style.display = 'none';
}

/** Rendert die Token-Liste im Plankton-Tab. */
function renderTokenList(container: HTMLElement, tokens: AgentToken[]): void {
  if (tokens.length === 0) {
    container.innerHTML = '<p class="prompt-token-hint">Keine Tokens vorhanden.</p>';
    return;
  }
  container.innerHTML = tokens.map(t => `
    <div class="prompt-token-row">
      <span class="prompt-token-name">${escapeHtml(t.name)}</span>
      <span class="prompt-token-role">${escapeHtml(t.role)}</span>
      <code class="prompt-token-value">${escapeHtml(t.token)}</code>
      <span class="prompt-token-status ${t.active ? 'active' : 'inactive'}">${t.active ? 'aktiv' : 'inaktiv'}</span>
    </div>
  `).join('');
}

/** Generiert die drei Markdown-Dateien und zeigt sie an. */
function generateFiles(): void {
  const url = (document.getElementById('prompt-plankton-url') as HTMLInputElement).value.trim() || window.location.origin;
  const projectName = state.project?.title || 'Plankton';

  const activeTokens = cachedTokens.filter(t => t.active);
  const tokenEntries = activeTokens.map(t => ({ name: t.name, token: t.token, role: t.role }));

  const secrets = generateSecretsMd(tokenEntries, url);
  const rules = generateRulesMd(url, projectName);
  const workflow = generateWorkflowMd();

  document.getElementById('prompt-out-secrets-pre')!.textContent = secrets;
  document.getElementById('prompt-out-rules-pre')!.textContent = rules;
  document.getElementById('prompt-out-workflow-pre')!.textContent = workflow;

  document.getElementById('prompt-output')!.style.display = '';
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

// === Hilfsfunktionen ===

/** Kopiert Text in die Zwischenablage mit visuellem Feedback. */
async function copyToClipboard(text: string, btn: HTMLElement): Promise<void> {
  try {
    await navigator.clipboard.writeText(text);
    const orig = btn.textContent;
    btn.textContent = '\u2713 Kopiert';
    setTimeout(() => { btn.textContent = orig; }, 1500);
  } catch { /* Clipboard nicht verfügbar */ }
}

/** Lädt einen String als Datei herunter. */
function downloadFile(filename: string, content: string): void {
  const blob = new Blob([content], { type: 'text/markdown;charset=utf-8' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

// === Bestehende Funktionen (unverändert) ===

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
  if (!await toastConfirm('Neues Projekt aus diesem JSON erstellen?')) return;
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

  if (!await toastConfirm('Projekt mit diesem JSON überschreiben?')) return;

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
