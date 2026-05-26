// Admin-Modal (Nutzerverwaltung + Tokens).

import { escapeHtml } from '../utils';
import { toastConfirm } from '../toast';
import { t } from '../i18n';
import type { AuthUser, AgentToken } from '../types';

interface AdminState {
  users: AuthUser[];
  editingUser: AuthUser | null;
  tokens: AgentToken[];
  tab: 'users' | 'tokens' | 'system';
}

let adminState: AdminState = { users: [], editingUser: null, tokens: [], tab: 'users' };

export { adminState };

export async function openAdminModal(): Promise<void> {
  adminState.tab = 'users';
  try {
    const r = await fetch('/api/admin/users');
    if (!r.ok) return;
    adminState.users = await r.json();
  } catch { return; }
  adminState.editingUser = null;
  updateAdminTabs();
  renderAdminUserList();
  document.getElementById('admin-user-form')!.style.display = 'none';
  document.getElementById('admin-user-list')!.style.display = '';
  document.getElementById('admin-list-actions')!.style.display = '';
  document.getElementById('admin-token-section')!.style.display = 'none';
  document.getElementById('admin-system-section')!.style.display = 'none';
  document.getElementById('admin-modal')!.classList.add('open');
}

function updateAdminTabs(): void {
  document.querySelectorAll<HTMLElement>('.admin-tab').forEach(el => {
    el.classList.toggle('admin-tab-active', el.dataset.tab === adminState.tab);
  });
}

export async function switchAdminTab(tab: 'users' | 'tokens' | 'system'): Promise<void> {
  adminState.tab = tab;
  updateAdminTabs();
  const showEl = (id: string, show: boolean) => {
    const el = document.getElementById(id);
    if (el) el.style.display = show ? '' : 'none';
  };
  showEl('admin-user-list', tab === 'users');
  showEl('admin-list-actions', tab === 'users');
  showEl('admin-user-form', false);
  showEl('admin-token-section', tab === 'tokens');
  showEl('admin-system-section', tab === 'system');
  if (tab === 'users') renderAdminUserList();
  else if (tab === 'tokens') await loadTokens();
  else if (tab === 'system') await loadSystemStatus();
}

async function loadTokens(): Promise<void> {
  try {
    const r = await fetch('/api/admin/tokens');
    if (!r.ok) return;
    adminState.tokens = await r.json();
  } catch { return; }
  renderTokenList();
}

function renderTokenList(): void {
  const el = document.getElementById('admin-token-list')!;
  if (adminState.tokens.length === 0) {
    el.innerHTML = `<div class="modal-list-empty">${t('admin.noTokens')}</div>`;
  } else {
    el.innerHTML = adminState.tokens.map(tk => `
      <div class="admin-user-row">
        <span class="admin-user-name">${escapeHtml(tk.name)}</span>
        <span class="admin-user-detail">${tk.role} ${tk.active === false ? '&middot; inaktiv' : ''}</span>
        <div class="admin-user-actions">
          <button class="btn-small" data-token-action="toggle" data-tid="${tk.id}">${tk.active ? t('admin.deactivate') : t('admin.activate')}</button>
          <button class="btn-small btn-danger-small" data-token-action="delete" data-tid="${tk.id}">${t('delete')}</button>
        </div>
      </div>
    `).join('');
  }
}

export async function createToken(): Promise<void> {
  const name = (document.getElementById('admin-token-name') as HTMLInputElement).value.trim();
  const role = (document.getElementById('admin-token-role') as HTMLSelectElement).value;
  if (!name) return;
  try {
    const r = await fetch('/api/admin/tokens', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name, role }),
    });
    if (!r.ok) return;
    const data = await r.json();
    const resultEl = document.getElementById('admin-token-result')!;
    resultEl.innerHTML = `<strong>${t('admin.tokenCreated')}</strong>\n\n${escapeHtml(data.token)}`;
    resultEl.style.display = '';
    (document.getElementById('admin-token-name') as HTMLInputElement).value = '';
    await loadTokens();
  } catch (err) {
    console.error('Token create error:', err);
  }
}

export function closeAdminModal(): void {
  document.getElementById('admin-modal')!.classList.remove('open');
}

function renderAdminUserList(): void {
  const el = document.getElementById('admin-user-list')!;
  if (adminState.users.length === 0) {
    el.innerHTML = `<div class="modal-list-empty">${t('admin.noUsers')}</div>`;
    return;
  }
  el.innerHTML = adminState.users.map(u => `
    <div class="admin-user-row">
      <span class="admin-user-name">${escapeHtml(u.display_name)}</span>
      <span class="admin-user-detail">${escapeHtml(u.username)} &middot; ${u.role}${u.active === false ? ' &middot; inaktiv' : ''}</span>
      <div class="admin-user-actions">
        <button class="btn-small" data-admin-action="edit" data-uid="${u.id}">${t('edit')}</button>
        <button class="btn-small" data-admin-action="reset-pw" data-uid="${u.id}">${t('admin.pwReset')}</button>
        <button class="btn-small btn-danger-small" data-admin-action="delete" data-uid="${u.id}">${t('delete')}</button>
      </div>
    </div>
  `).join('');
}

export function showAdminForm(user?: AuthUser | null): void {
  adminState.editingUser = user || null;
  document.getElementById('admin-user-list')!.style.display = 'none';
  document.getElementById('admin-list-actions')!.style.display = 'none';
  document.getElementById('admin-user-form')!.style.display = '';
  const usernameInput = document.getElementById('admin-username') as HTMLInputElement;
  usernameInput.value = user ? user.username : '';
  usernameInput.disabled = !!user;
  (document.getElementById('admin-displayname') as HTMLInputElement).value = user ? user.display_name : '';
  const pwInput = document.getElementById('admin-password') as HTMLInputElement;
  pwInput.value = '';
  pwInput.placeholder = user ? t('admin.unchanged') : 'Passwort';
  (document.getElementById('admin-role') as HTMLSelectElement).value = user ? user.role : 'user';
  setTimeout(() => (document.getElementById(user ? 'admin-displayname' : 'admin-username') as HTMLInputElement).focus(), 50);
}

export async function saveAdminForm(): Promise<void> {
  const username = (document.getElementById('admin-username') as HTMLInputElement).value.trim();
  const displayName = (document.getElementById('admin-displayname') as HTMLInputElement).value.trim();
  const password = (document.getElementById('admin-password') as HTMLInputElement).value;
  const role = (document.getElementById('admin-role') as HTMLSelectElement).value;
  if (!username || !displayName) return;

  try {
    if (adminState.editingUser) {
      await fetch(`/api/admin/users/${adminState.editingUser.id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ display_name: displayName, role, active: true }),
      });
    } else {
      if (!password) return;
      await fetch('/api/admin/users', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ username, display_name: displayName, password, role }),
      });
    }
    await openAdminModal();
  } catch (err) {
    console.error('Admin save error:', err);
  }
}

export async function handleTokenAction(action: string, tid: string): Promise<void> {
  if (action === 'delete') {
    if (!await toastConfirm('Token löschen?')) return;
    await fetch(`/api/admin/tokens/${tid}`, { method: 'DELETE' });
    await loadTokens();
  } else if (action === 'toggle') {
    const token = adminState.tokens.find(tk => tk.id === tid);
    if (!token) return;
    await fetch(`/api/admin/tokens/${tid}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ active: !token.active }),
    });
    await loadTokens();
  }
}

async function loadSystemStatus(): Promise<void> {
  const el = document.getElementById('admin-system-content');
  if (!el) return;
  el.innerHTML = `<span class="text-text-dim text-xs font-mono">${t('admin.systemLoading')}</span>`;
  try {
    const r = await fetch('/api/admin/system-status');
    if (!r.ok) throw new Error(String(r.status));
    const data = await r.json();
    const fmt = (iso: string | null) => {
      if (!iso) return t('admin.systemNever');
      const d = new Date(iso);
      return d.toLocaleString();
    };
    const now = Date.now();
    const nextMs = new Date(data.next_maintenance_run).getTime();
    const diffSec = Math.max(0, Math.round((nextMs - now) / 1000));
    const h = Math.floor(diffSec / 3600);
    const m = Math.floor((diffSec % 3600) / 60);
    const s = diffSec % 60;
    const countdown = `${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
    el.innerHTML = `
      <div class="flex flex-col gap-2 text-[13px] font-mono">
        <div class="flex items-center gap-3">
          <span class="text-text-dim uppercase text-xs tracking-wide w-36">${t('admin.systemLastRun')}</span>
          <span class="text-text">${fmt(data.last_maintenance_run)}</span>
        </div>
        <div class="flex items-center gap-3">
          <span class="text-text-dim uppercase text-xs tracking-wide w-36">${t('admin.systemNextRun')}</span>
          <span class="text-text">${fmt(data.next_maintenance_run)}</span>
        </div>
        <div class="flex items-center gap-3">
          <span class="text-text-dim uppercase text-xs tracking-wide w-36">${t('admin.systemCountdown')}</span>
          <span class="text-accent font-semibold tabular-nums">${countdown}</span>
        </div>
        <div class="flex items-center gap-3">
          <span class="text-text-dim uppercase text-xs tracking-wide w-36">${t('admin.systemInterval')}</span>
          <span class="text-text">${data.interval_seconds / 60} min</span>
        </div>
      </div>
    `;
  } catch {
    el.innerHTML = `<span class="text-[#ff6b6b] text-xs font-mono">${t('admin.systemError')}</span>`;
  }
}

export async function handleAdminUserAction(action: string, uid: string): Promise<void> {
  if (action === 'edit') {
    const user = adminState.users.find(u => u.id === uid);
    if (user) showAdminForm(user);
  } else if (action === 'delete') {
    if (!await toastConfirm('Nutzer löschen?')) return;
    await fetch(`/api/admin/users/${uid}`, { method: 'DELETE' });
    await openAdminModal();
  } else if (action === 'reset-pw') {
    const newPw = prompt(t('admin.newPasswordPrompt'));
    if (!newPw) return;
    await fetch(`/api/admin/users/${uid}/reset-password`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ new_password: newPw }),
    });
    alert(t('admin.passwordReset'));
  }
}
