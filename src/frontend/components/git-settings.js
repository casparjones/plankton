// Git-Einstellungen Modal – Konfiguration und Sync.

import api from '../api.js';
import { state } from '../state.js';
import { formatDate } from '../utils.js';

/// Aktualisiert das Git-Status-Icon im Board-Header.
export function updateGitStatusIcon() {
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
    icon.title = 'Git-Sync deaktiviert';
    icon.innerHTML = '&#128268;';
  } else if (git.last_error) {
    icon.className = 'git-status-icon git-icon-error';
    icon.title = `Git-Fehler: ${git.last_error}`;
    icon.innerHTML = '&#128268;';
  } else if (git.last_push) {
    icon.className = 'git-status-icon git-icon-ok';
    icon.title = `Letzter Git-Push: ${formatDate(git.last_push)}`;
    icon.innerHTML = '&#128268;';
  } else {
    icon.className = 'git-status-icon git-icon-disabled';
    icon.title = 'Git konfiguriert, noch kein Push';
    icon.innerHTML = '&#128268;';
  }
}

export async function openGitModal() {
  if (!state.project) return;
  const config = await api.get(`/api/projects/${state.project._id}/git`);

  document.getElementById('git-repo-url').value = config?.repo_url || '';
  document.getElementById('git-branch').value = config?.branch || 'main';
  document.getElementById('git-path').value = config?.path || 'plankton.json';
  document.getElementById('git-enabled').checked = config?.enabled || false;
  renderGitStatus(config);
  document.getElementById('git-modal').classList.add('open');
}

export function closeGitModal() {
  document.getElementById('git-modal').classList.remove('open');
}

function renderGitStatus(config) {
  const el = document.getElementById('git-status');
  if (!config) {
    el.innerHTML = '<div class="git-status-info">Noch nicht konfiguriert</div>';
    return;
  }
  let html = '';
  if (config.last_push) {
    html += `<div class="git-status-ok">Letzter Push: ${formatDate(config.last_push)}</div>`;
  }
  if (config.last_error) {
    html += `<div class="git-status-error">Fehler: ${config.last_error}</div>`;
  }
  if (!config.last_push && !config.last_error) {
    html += '<div class="git-status-info">Noch kein Sync durchgeführt</div>';
  }
  el.innerHTML = html;
}

export async function saveGitConfig() {
  if (!state.project) return;
  const config = {
    repo_url: document.getElementById('git-repo-url').value.trim(),
    branch: document.getElementById('git-branch').value.trim() || 'main',
    path: document.getElementById('git-path').value.trim() || 'plankton.json',
    enabled: document.getElementById('git-enabled').checked,
  };
  if (!config.repo_url) {
    alert('Repository-URL ist erforderlich');
    return;
  }
  await api.put(`/api/projects/${state.project._id}/git`, config);
  closeGitModal();
}

export async function triggerGitSync() {
  if (!state.project) return;
  const btn = document.getElementById('git-sync-btn');
  btn.textContent = 'Synchronisiere…';
  btn.disabled = true;
  try {
    const result = await api.post(`/api/projects/${state.project._id}/git/sync`, {});
    if (result.success) {
      btn.textContent = 'Erfolgreich!';
      // Aktualisiere Status
      const config = await api.get(`/api/projects/${state.project._id}/git`);
      renderGitStatus(config);
    } else {
      btn.textContent = 'Fehlgeschlagen';
      const config = await api.get(`/api/projects/${state.project._id}/git`);
      renderGitStatus(config);
    }
  } catch (err) {
    btn.textContent = 'Fehler!';
    document.getElementById('git-status').innerHTML =
      `<div class="git-status-error">Fehler: ${err.message}</div>`;
  }
  setTimeout(() => {
    btn.textContent = 'Jetzt synchronisieren';
    btn.disabled = false;
  }, 2000);
}
