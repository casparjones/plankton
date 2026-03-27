// Git-Einstellungen Modal – Konfiguration und Sync.

import api from '../api';
import { state } from '../state';
import { formatDate } from '../utils';
import { t } from '../i18n';
import type { GitConfig } from '../types';

/// Aktualisiert das Git-Status-Icon im Board-Header.
export function updateGitStatusIcon(): void {
  const icon = document.getElementById('git-status-icon');
  if (!icon) return;
  const git = state.project?.git;
  if (!git) {
    icon.style.display = 'none';
    return;
  }
  icon.style.display = '';
  if (!git.enabled) {
    icon.className = 'git-status-icon git-icon-disabled';
    icon.title = t('git.disabled');
    icon.innerHTML = '&#128268;';
  } else if (git.last_error) {
    icon.className = 'git-status-icon git-icon-error';
    icon.title = t('git.error', { error: git.last_error });
    icon.innerHTML = '&#128268;';
  } else if (git.last_push) {
    icon.className = 'git-status-icon git-icon-ok';
    icon.title = t('git.lastPush', { date: formatDate(git.last_push) });
    icon.innerHTML = '&#128268;';
  } else {
    icon.className = 'git-status-icon git-icon-disabled';
    icon.title = t('git.configured');
    icon.innerHTML = '&#128268;';
  }
}

export async function openGitModal(): Promise<void> {
  if (!state.project) return;
  const config = await api.get<GitConfig>(`/api/projects/${state.project._id}/git`);

  (document.getElementById('git-repo-url') as HTMLInputElement).value = config?.repo_url || '';
  (document.getElementById('git-branch') as HTMLInputElement).value = config?.branch || 'main';
  (document.getElementById('git-path') as HTMLInputElement).value = config?.path || 'plankton.json';
  (document.getElementById('git-enabled') as HTMLInputElement).checked = config?.enabled || false;
  renderGitStatus(config);
  document.getElementById('git-modal')!.classList.add('open');
}

export function closeGitModal(): void {
  document.getElementById('git-modal')!.classList.remove('open');
}

function renderGitStatus(config: GitConfig | null): void {
  const el = document.getElementById('git-status')!;
  if (!config) {
    el.innerHTML = `<div class="git-status-info">${t('git.notConfigured')}</div>`;
    return;
  }
  let html = '';
  if (config.last_push) {
    html += `<div class="git-status-ok">${t('git.lastPushStatus', { date: formatDate(config.last_push) })}</div>`;
  }
  if (config.last_error) {
    html += `<div class="git-status-error">${t('git.errorStatus', { error: config.last_error })}</div>`;
  }
  if (!config.last_push && !config.last_error) {
    html += `<div class="git-status-info">${t('git.noSync')}</div>`;
  }
  el.innerHTML = html;
}

export async function saveGitConfig(): Promise<void> {
  if (!state.project) return;
  const config = {
    repo_url: (document.getElementById('git-repo-url') as HTMLInputElement).value.trim(),
    branch: (document.getElementById('git-branch') as HTMLInputElement).value.trim() || 'main',
    path: (document.getElementById('git-path') as HTMLInputElement).value.trim() || 'plankton.json',
    enabled: (document.getElementById('git-enabled') as HTMLInputElement).checked,
  };
  if (!config.repo_url) {
    alert(t('git.repoRequired'));
    return;
  }
  await api.put(`/api/projects/${state.project._id}/git`, config);
  closeGitModal();
}

export async function triggerGitSync(): Promise<void> {
  if (!state.project) return;
  const btn = document.getElementById('git-sync-btn') as HTMLButtonElement;
  btn.textContent = t('git.syncing');
  btn.disabled = true;
  try {
    const result = await api.post<{ success: boolean }>(`/api/projects/${state.project._id}/git/sync`, {});
    if (result.success) {
      btn.textContent = t('git.syncSuccess');
      // Aktualisiere Status
      const config = await api.get<GitConfig>(`/api/projects/${state.project._id}/git`);
      renderGitStatus(config);
    } else {
      btn.textContent = t('git.syncFailed');
      const config = await api.get<GitConfig>(`/api/projects/${state.project._id}/git`);
      renderGitStatus(config);
    }
  } catch (err: any) {
    btn.textContent = t('git.syncError');
    document.getElementById('git-status')!.innerHTML =
      `<div class="git-status-error">${t('git.errorStatus', { error: err.message })}</div>`;
  }
  setTimeout(() => {
    btn.textContent = t('git.syncNow');
    btn.disabled = false;
  }, 2000);
}
