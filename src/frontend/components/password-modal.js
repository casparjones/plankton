// Passwort-Ändern Modal.

import { state } from '../state.js';
import { checkAuth, doChangePassword } from './auth.js';

export function openPasswordModal(force) {
  document.getElementById('pw-error').textContent = '';
  document.getElementById('pw-old').value = '';
  document.getElementById('pw-new').value = '';
  document.getElementById('pw-confirm').value = '';
  const closeBtn = document.getElementById('pw-modal-close');
  closeBtn.style.display = force ? 'none' : '';
  document.getElementById('password-modal').dataset.force = force ? '1' : '';
  document.getElementById('password-modal').classList.add('open');
  setTimeout(() => document.getElementById('pw-old').focus(), 50);
}

export function closePasswordModal() {
  if (document.getElementById('password-modal').dataset.force === '1') return;
  document.getElementById('password-modal').classList.remove('open');
}

export async function savePassword() {
  const oldPw = document.getElementById('pw-old').value;
  const newPw = document.getElementById('pw-new').value;
  const confirmPw = document.getElementById('pw-confirm').value;
  const errEl = document.getElementById('pw-error');
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
    document.getElementById('password-modal').dataset.force = '';
    document.getElementById('password-modal').classList.remove('open');
    const user = await checkAuth();
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
  } catch (err) {
    errEl.textContent = err.message;
  }
}
