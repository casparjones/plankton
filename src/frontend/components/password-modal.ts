// Passwort-Ändern Modal.

import { state } from '../state';
import { checkAuth, doChangePassword } from './auth';
import type { Claims } from '../types';

export function openPasswordModal(force: boolean): void {
  document.getElementById('pw-error')!.textContent = '';
  (document.getElementById('pw-old') as HTMLInputElement).value = '';
  (document.getElementById('pw-new') as HTMLInputElement).value = '';
  (document.getElementById('pw-confirm') as HTMLInputElement).value = '';
  const closeBtn = document.getElementById('pw-modal-close') as HTMLElement;
  closeBtn.style.display = force ? 'none' : '';
  (document.getElementById('password-modal') as HTMLElement).dataset.force = force ? '1' : '';
  document.getElementById('password-modal')!.classList.add('open');
  setTimeout(() => (document.getElementById('pw-old') as HTMLInputElement).focus(), 50);
}

export function closePasswordModal(): void {
  if ((document.getElementById('password-modal') as HTMLElement).dataset.force === '1') return;
  document.getElementById('password-modal')!.classList.remove('open');
}

export async function savePassword(): Promise<void> {
  const oldPw = (document.getElementById('pw-old') as HTMLInputElement).value;
  const newPw = (document.getElementById('pw-new') as HTMLInputElement).value;
  const confirmPw = (document.getElementById('pw-confirm') as HTMLInputElement).value;
  const errEl = document.getElementById('pw-error')!;
  errEl.textContent = '';

  if (newPw !== confirmPw) {
    errEl.textContent = 'Passwörter stimmen nicht überein';
    return;
  }
  if (newPw.length < 4) {
    errEl.textContent = 'Passwort muss mindestens 4 Zeichen haben';
    return;
  }
  try {
    await doChangePassword(oldPw, newPw);
    (document.getElementById('password-modal') as HTMLElement).dataset.force = '';
    document.getElementById('password-modal')!.classList.remove('open');
    const user = await checkAuth() as Claims | null;
    if (user) {
      state.currentUser = user;
      // Update user section in sidebar
      const avatarEl = document.getElementById('user-avatar');
      const nameEl = document.getElementById('user-name');
      const roleEl = document.getElementById('user-role');
      if (avatarEl) avatarEl.textContent = (user.display_name || user.username || '?')[0].toUpperCase();
      if (nameEl) nameEl.textContent = user.display_name || user.username;
      if (roleEl) roleEl.textContent = user.role;
    }
  } catch (err: any) {
    errEl.textContent = err.message;
  }
}
