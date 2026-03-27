// Auth-Funktionen (Login, Logout, Session).

import { state } from '../state';
import { t } from '../i18n';
import type { Claims } from '../types';

export async function checkAuth(): Promise<Claims | null> {
  try {
    const r = await fetch('/auth/me');
    if (!r.ok) return null;
    return await r.json() as Claims;
  } catch {
    return null;
  }
}

export async function doLogin(username: string, password: string): Promise<Claims> {
  const r = await fetch('/auth/login', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ username, password }),
  });
  if (!r.ok) {
    const err = await r.json().catch(() => ({ error: t('auth.loginFailed') }));
    throw new Error(err.error || t('auth.loginFailed'));
  }
  return await r.json() as Claims;
}

export async function doLogout(showLoginPage: () => void): Promise<void> {
  await fetch('/auth/logout', { method: 'POST' });
  state.currentUser = null;
  showLoginPage();
}

export async function doChangePassword(oldPassword: string, newPassword: string): Promise<unknown> {
  const r = await fetch('/auth/change-password', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ old_password: oldPassword, new_password: newPassword }),
  });
  if (!r.ok) {
    const err = await r.json().catch(() => ({ error: t('error') }));
    throw new Error(err.error || t('passwordModal.changeFailed'));
  }
  return await r.json();
}

export function updateUserSection(): void {
  const user = state.currentUser;
  if (!user) return;
  const avatarEl = document.getElementById('user-avatar');
  const nameEl = document.getElementById('user-name');
  const roleEl = document.getElementById('user-role');
  const adminBtn = document.getElementById('admin-btn');

  if (avatarEl) avatarEl.textContent = (user.display_name || user.username || '?')[0].toUpperCase();
  if (nameEl) nameEl.textContent = user.display_name || user.username;
  if (roleEl) roleEl.textContent = user.role;
  if (adminBtn) adminBtn.style.display = user.role === 'admin' ? '' : 'none';
}
