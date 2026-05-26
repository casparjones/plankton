// Projekt-Menü (Dropdown, Editieren, JSON Import/Export, Prompt mit Tabs).

import api from '../api';
import { state } from '../state';
import { escapeHtml } from '../utils';
import { t } from '../i18n';
import { renderBoard } from './board';
import { renderJsonTree, toggleJsonView } from './json-view';
import { loadProjects, renameProject } from '../services/project-service';
import { updateProjectTitle } from './sidebar';
import { subscribeSSE } from '../services/sse-service';
import { toastConfirm } from '../toast';
import { openGitModal } from './git-settings';
import { generateRulesMd, generateWorkflowMd } from './prompt-generator';
import type { ProjectDoc, AgentToken } from '../types';

// Zwischenspeicher für geladene Tokens.
let cachedTokens: AgentToken[] = [];
// Aktuell sichtbarer Output-Tab.
let activeOutputTab = 'setup';

export function openProjectDropdown(): void {
  closeProjectDropdown();
  if (!state.project) return;

  const dropdown = document.getElementById('project-dropdown')!;
  dropdown.innerHTML = `
    <button class="proj-dropdown-item" data-action="edit">&#9998; ${t('project.editProject')}</button>
    <button class="proj-dropdown-item" data-action="prompt">&#9733; ${t('prompt.aiAgents')}</button>
    <button class="proj-dropdown-item" data-action="cli">&#9881; ${t('prompt.installCli')}</button>
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
  const url = window.location.origin;
  // Simple-Tab: Prompt generieren.
  const prompt = generateProjectPrompt();
  document.getElementById('prompt-content')!.textContent = prompt;
  // Plankton-Tab: URL vorbelegen.
  const urlInput = document.getElementById('prompt-plankton-url') as HTMLInputElement;
  if (urlInput && !urlInput.value) {
    urlInput.value = url;
  }
  // claude.ai Tab: Server-URL setzen.
  const serverUrl = document.getElementById('claudeai-server-url');
  if (serverUrl) serverUrl.textContent = `${url}/mcp`;
  document.getElementById('prompt-modal')!.classList.add('open');
  // Tokens laden wenn Plankton-Tab aktiv ist oder beim ersten Öffnen.
  loadTokensForPrompt();
}

export function closePromptModal(): void {
  document.getElementById('prompt-modal')!.classList.remove('open');
}

export function openCliModal(): void {
  const url = window.location.origin;
  const installCmd = document.getElementById('cli-install-cmd');
  if (installCmd) installCmd.textContent = `curl -fsSL ${url}/install | bash`;
  const loginCmd = document.getElementById('cli-login-cmd');
  if (loginCmd) loginCmd.textContent = `plankton remote add origin ${url}`;
  const skillCmd = document.getElementById('cli-skill-cmd');
  if (skillCmd) skillCmd.textContent = `plankton skill install ${url} --global`;
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
      activeOutputTab = (tab as HTMLElement).dataset.outputTab || 'setup';
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

/** Rendert die Token-Liste im Plankton-Tab (maskiert, keine Secrets). */
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
  `).join('') + `
    <p class="prompt-token-hint" style="margin-top:8px">
      Tokens werden aus Sicherheitsgr&uuml;nden maskiert angezeigt.<br>
      Verwende die CLI f&uuml;r die Einrichtung: <code>plankton skill install ${window.location.origin} --global</code>
    </p>`;
}

/** Generiert die Konfigurations-Dateien und zeigt sie an. */
function generateFiles(): void {
  const url = (document.getElementById('prompt-plankton-url') as HTMLInputElement).value.trim() || window.location.origin;
  const projectName = state.project?.title || 'Plankton';

  const rules = generateRulesMd(url, projectName);
  const workflow = generateWorkflowMd();

  // CLI-Setup statt secrets.md
  const setupText = `# CLI installieren\ncurl -fsSL ${url}/install | bash\n\n# Skill installieren (Login + Secrets automatisch)\nplankton skill install ${url} --global`;
  document.getElementById('prompt-out-setup-pre')!.textContent = setupText;
  document.getElementById('prompt-out-rules-pre')!.textContent = rules;
  document.getElementById('prompt-out-workflow-pre')!.textContent = workflow;

  document.getElementById('prompt-output')!.style.display = '';
}

function generateProjectPrompt(): string {
  const p = state.project!;

  return `Du bist ein Projektmanagement-Assistent. Generiere Tasks als JSON für das Kanban-Board "${p.title}".

## Task-Format

Antworte mit einem JSON-Array. Tasks werden automatisch in "Todo" angelegt.

[
  {
    "id": "eindeutige-id",
    "title": "Task-Titel",
    "description": "Beschreibung mit Akzeptanzkriterien",
    "task_type": "task",
    "blocks": [],
    "blocked_by": [],
    "subtask_ids": []
  }
]

- **task_type**: "task" (Standard), "epic" (mit Subtasks) oder "job" (automatisiert)
- **blocks** / **blocked_by**: IDs anderer Tasks für Abhängigkeiten
- **subtask_ids**: IDs von Subtasks (nur bei Epics)

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

// === Tab-Navigation für Project Settings ===

/** Initialisiert die Tab-Navigation im Project-Settings-Modal. */
export function initProjectSettingsTabs(): void {
  const modal = document.getElementById('project-modal');
  if (!modal) return;

  modal.addEventListener('click', (e: Event) => {
    const tab = (e.target as HTMLElement).closest<HTMLElement>('[data-proj-tab]');
    if (!tab) return;
    const tabName = tab.dataset.projTab!;
    switchProjectTab(tabName);
  });
}

function switchProjectTab(tabName: string): void {
  // Tab-Buttons umschalten.
  document.querySelectorAll('.proj-settings-tab').forEach(btn => {
    btn.classList.toggle('proj-settings-tab-active', (btn as HTMLElement).dataset.projTab === tabName);
  });
  // Tab-Inhalte umschalten.
  document.querySelectorAll('.proj-settings-tab-content').forEach(content => {
    (content as HTMLElement).classList.add('hidden');
  });
  document.getElementById(`proj-${tabName}-tab`)?.classList.remove('hidden');
}

// === Bestehende Funktionen (angepasst) ===

export async function openProjectMenu(): Promise<void> {
  if (!state.project) return;
  const project = await api.get<ProjectDoc>(`/api/projects/${state.project._id}?include_archived=true`);

  // Tab 1: Details befüllen.
  (document.getElementById('proj-field-id') as HTMLInputElement).value = project._id || '';
  (document.getElementById('proj-field-title') as HTMLInputElement).value = project.title || '';
  (document.getElementById('proj-field-slug') as HTMLInputElement).value = project.slug || '';
  (document.getElementById('proj-field-owner') as HTMLInputElement).value = project.owner || '';

  // Type-Dropdown: Option "list" deaktivieren wenn mehr als eine Spalte.
  const typeSelect = document.getElementById('proj-field-type') as HTMLSelectElement;
  typeSelect.value = project.type || 'kanban';
  const visibleCols = (project.columns || []).filter(c => !c.hidden);
  const listOption = typeSelect.querySelector('option[value="list"]') as HTMLOptionElement | null;
  const tooltip = document.getElementById('proj-type-tooltip');
  if (listOption) {
    const canSwitchToList = visibleCols.length <= 1 || (project.type === 'list');
    listOption.disabled = !canSwitchToList;
    if (tooltip) {
      tooltip.classList.toggle('hidden', canSwitchToList);
    }
  }
  typeSelect.addEventListener('mouseover', () => {
    if (listOption?.disabled && tooltip) tooltip.classList.remove('hidden');
  });
  typeSelect.addEventListener('mouseout', () => {
    if (tooltip) tooltip.classList.add('hidden');
  });

  // Pinned-Checkbox befüllen.
  (document.getElementById('proj-field-pinned') as HTMLInputElement).checked = !!project.pinned;
  // Automatisierungs-Felder befüllen (Defaults: 10 / 90 wenn nicht gesetzt).
  (document.getElementById('proj-field-done-expire') as HTMLInputElement).value =
    project.doneExpire != null ? String(project.doneExpire) : '10';
  (document.getElementById('proj-field-archive-delete') as HTMLInputElement).value =
    project.archiveDelete != null ? String(project.archiveDelete) : '90';

  // Tab 2: Users befüllen.
  renderUserList(project.users || []);

  // Tab 3: JSON befüllen.
  (document.getElementById('proj-modal-json') as HTMLTextAreaElement).value = JSON.stringify(project, null, 2);
  renderJsonTree(project, document.getElementById('proj-json-tree')!);
  document.getElementById('proj-json-tree')!.style.display = '';
  (document.getElementById('proj-modal-json') as HTMLTextAreaElement).style.display = 'none';
  const toggleBtn = document.getElementById('proj-view-toggle')!;
  toggleBtn.textContent = 'Raw JSON';

  // Immer mit Details-Tab starten.
  switchProjectTab('details');

  document.getElementById('project-modal')!.classList.add('open');
}

/** Rendert die Nutzerliste im Users-Tab. */
function renderUserList(users: { id: string; name: string; avatar: string; role: string }[]): void {
  const list = document.getElementById('proj-user-list')!;
  const placeholder = document.getElementById('proj-users-placeholder')!;
  if (users.length === 0) {
    list.innerHTML = '';
    placeholder.style.display = '';
    return;
  }
  placeholder.style.display = 'none';
  list.innerHTML = users.map(u => `
    <div class="flex items-center gap-2 px-2 py-1 bg-surface-2 border border-border rounded-md text-sm" data-user-id="${escapeHtml(u.id)}">
      <span class="flex-1 text-text">${escapeHtml(u.name || u.id)}</span>
      <span class="text-text-dim text-xs font-mono">${escapeHtml(u.role || '')}</span>
      <button class="proj-user-remove bg-transparent border-none text-text-dim cursor-pointer text-base px-1 hover:text-danger" title="Nutzer entfernen" data-user-id="${escapeHtml(u.id)}">&#10005;</button>
    </div>
  `).join('');

  // Remove-Buttons.
  list.querySelectorAll('.proj-user-remove').forEach(btn => {
    btn.addEventListener('click', async () => {
      const userId = (btn as HTMLElement).dataset.userId!;
      await removeProjectUser(userId);
    });
  });

  // Plus-Button verdrahten (nur einmal).
  const addBtn = document.getElementById('proj-user-add');
  if (addBtn && !addBtn.dataset.wired) {
    addBtn.dataset.wired = '1';
    addBtn.addEventListener('click', addProjectUser);
  }
}

export function closeProjectMenu(): void {
  document.getElementById('project-modal')!.classList.remove('open');
}

/** Speichert die Details-Tab-Felder (id/readonly, title, type, slug, owner). */
export async function saveProjectDetails(): Promise<void> {
  if (!state.project) return;
  const project = await api.get<ProjectDoc>(`/api/projects/${state.project._id}?include_archived=true`);

  project.title = (document.getElementById('proj-field-title') as HTMLInputElement).value.trim() || project.title;
  project.slug = (document.getElementById('proj-field-slug') as HTMLInputElement).value.trim() || project.slug;
  project.owner = (document.getElementById('proj-field-owner') as HTMLInputElement).value.trim() || null;
  project.type = (document.getElementById('proj-field-type') as HTMLSelectElement).value;
  project.pinned = (document.getElementById('proj-field-pinned') as HTMLInputElement).checked || undefined;

  const doneExpireRaw = (document.getElementById('proj-field-done-expire') as HTMLInputElement).value.trim();
  project.doneExpire = doneExpireRaw !== '' ? parseInt(doneExpireRaw, 10) : null;
  const archiveDeleteRaw = (document.getElementById('proj-field-archive-delete') as HTMLInputElement).value.trim();
  project.archiveDelete = archiveDeleteRaw !== '' ? parseInt(archiveDeleteRaw, 10) : null;

  try {
    state.project = await api.put<ProjectDoc>(`/api/projects/${state.project._id}`, project);
    await loadProjects();
    closeProjectMenu();
    renderBoard();
    updateProjectTitle();
  } catch (err: any) {
    alert('Fehler beim Speichern: ' + err.message);
  }
}

/** Fügt einen Nutzer zum Projekt hinzu (aus dem Users-Tab). */
async function addProjectUser(): Promise<void> {
  if (!state.project) return;
  const input = document.getElementById('proj-user-input') as HTMLInputElement;
  const name = input.value.trim();
  if (!name) return;

  const project = await api.get<ProjectDoc>(`/api/projects/${state.project._id}?include_archived=true`);
  const users = project.users || [];
  users.push({ id: crypto.randomUUID(), name, avatar: '', role: 'member' });
  project.users = users;

  try {
    state.project = await api.put<ProjectDoc>(`/api/projects/${state.project._id}`, project);
    input.value = '';
    renderUserList(state.project.users || []);
    await loadProjects();
  } catch (err: any) {
    alert('Fehler: ' + err.message);
  }
}

/** Entfernt einen Nutzer aus dem Projekt. */
async function removeProjectUser(userId: string): Promise<void> {
  if (!state.project) return;
  const project = await api.get<ProjectDoc>(`/api/projects/${state.project._id}?include_archived=true`);
  project.users = (project.users || []).filter(u => u.id !== userId);

  try {
    state.project = await api.put<ProjectDoc>(`/api/projects/${state.project._id}`, project);
    renderUserList(state.project.users || []);
    await loadProjects();
  } catch (err: any) {
    alert('Fehler: ' + err.message);
  }
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
  const text = textarea.value.trim();
  if (!text) return;

  let data: any;
  try {
    data = JSON.parse(text);
  } catch {
    alert('Ungültiges JSON');
    return;
  }

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

/** @deprecated Titel-Speicherung erfolgt jetzt über saveProjectDetails() (Tab 1). */
export async function saveProjectTitle(): Promise<void> {
  if (!state.project) return;
  const titleInput = document.getElementById('proj-field-title') as HTMLInputElement;
  if (!titleInput) return;
  const newTitle = titleInput.value.trim();
  if (newTitle && newTitle !== state.project.title) {
    await renameProject(state.project._id, newTitle);
  }
}
