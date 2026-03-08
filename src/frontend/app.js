// Plankton Frontend – Anwendungslogik (Orchestrierung).

import { state } from './state.js';
import { buildDOM } from './dom.js';
import { initTheme, toggleTheme } from './components/theme.js';
import { checkAuth, doLogin, updateUserSection } from './components/auth.js';
import { loadProjects, openProject, getLastProject } from './services/project-service.js';
import { openPasswordModal } from './components/password-modal.js';

function showLoginPage() {
  document.body.innerHTML = `
    <div class="login-page">
      <div class="login-card">
        <div class="login-logo">🪼 Plankton</div>
        <div id="login-error" class="login-error"></div>
        <form id="login-form">
          <label>Username
            <input id="login-username" type="text" autocomplete="username" autofocus />
          </label>
          <label>Passwort
            <input id="login-password" type="password" autocomplete="current-password" />
          </label>
          <button type="submit" class="btn-primary login-btn">Anmelden</button>
        </form>
      </div>
    </div>
  `;
  document.getElementById('login-form').addEventListener('submit', async (e) => {
    e.preventDefault();
    const username = document.getElementById('login-username').value.trim();
    const password = document.getElementById('login-password').value;
    const errEl = document.getElementById('login-error');
    errEl.textContent = '';
    try {
      await doLogin(username, password);
      const user = await checkAuth();
      if (user) {
        state.currentUser = user;
        if (user.must_change_password) {
          await startApp();
          setTimeout(() => openPasswordModal(true), 100);
        } else {
          await startApp();
        }
      }
    } catch (err) {
      errEl.textContent = err.message;
    }
  });
}

async function startApp() {
  buildDOM(showLoginPage);
  initTheme();
  document.getElementById('theme-toggle').addEventListener('click', toggleTheme);
  updateUserSection();
  await loadProjects();
  if (state.projects.length > 0) {
    const lastId = getLastProject();
    const target = lastId && state.projects.find(p => p._id === lastId)
        ? lastId
        : state.projects[0]._id;
    await openProject(target);
  }
}

export async function init() {
  const user = await checkAuth();
  if (!user) {
    showLoginPage();
    return;
  }
  state.currentUser = user;
  await startApp();
  if (user.must_change_password) {
    setTimeout(() => openPasswordModal(true), 100);
  }
}
