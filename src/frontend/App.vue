<script setup lang="ts">
// Plankton – Root-Komponente.
// Steuert die Hauptansicht: Login-Screen oder Board (via AppLayout).

import { ref, onMounted, nextTick } from 'vue'
import { useTheme } from './composables/useTheme'
import AppLayout from './components/AppLayout.vue'
import type { Claims } from './types'

import { checkAuth, doLogin, updateUserSection } from './components/auth'
// @ts-ignore
import { loadProjects, openProject, getLastProject } from './services/project-service'
// @ts-ignore
import { openPasswordModal } from './components/password-modal'
import { state } from './state'
import { toggleTheme as legacyToggleTheme } from './components/theme'

const { initTheme } = useTheme()

const authChecked = ref(false)
const isAuthenticated = ref(false)
const loginError = ref('')
const loginUsername = ref('')
const loginPassword = ref('')

/** Startet die Board-Ansicht nach erfolgreichem Login oder Auth-Check. */
async function startApp(): Promise<void> {
  console.log('[App] startApp() called')
  await nextTick()

  initTheme()

  // Theme-Toggle aus dem AppLayout-DOM verbinden
  const themeToggle = document.getElementById('theme-toggle')
  if (themeToggle) {
    themeToggle.addEventListener('click', legacyToggleTheme)
  }

  updateUserSection()
  console.log('[App] loadProjects() ...')
  await loadProjects()
  console.log('[App] loadProjects() done, state.projects:', state.projects?.length, state.projects)

  if (state.projects.length > 0) {
    const lastId = getLastProject()
    const target = lastId && state.projects.find((p: { _id: string }) => p._id === lastId)
      ? lastId
      : state.projects[0]._id
    console.log('[App] openProject()', target)
    await openProject(target)
    console.log('[App] openProject() done, state.project:', state.project?._id, state.project?.title)
    console.log('[App] state.project.columns:', state.project?.columns?.length, state.project?.columns)
    console.log('[App] state.project.tasks:', state.project?.tasks?.length, state.project?.tasks)
  } else {
    console.log('[App] Keine Projekte vorhanden')
  }
}

/** Zeigt die Login-Seite (setzt Vue-State zurück). */
function showLogin(): void {
  isAuthenticated.value = false
  loginError.value = ''
  loginUsername.value = ''
  loginPassword.value = ''
}

/** Verarbeitet den Login-Submit. */
async function handleLogin(): Promise<void> {
  loginError.value = ''
  try {
    await doLogin(loginUsername.value.trim(), loginPassword.value)
    const user: Claims | null = await checkAuth()
    if (user) {
      state.currentUser = user
      isAuthenticated.value = true

      await nextTick()
      await startApp()

      if (user.must_change_password) {
        setTimeout(() => openPasswordModal(true), 100)
      }
    }
  } catch (err: unknown) {
    loginError.value = err instanceof Error ? err.message : 'Anmeldung fehlgeschlagen'
  }
}

onMounted(async () => {
  console.log('[App] onMounted, checking auth...')
  const user: Claims | null = await checkAuth()
  console.log('[App] checkAuth result:', user)
  if (!user) {
    console.log('[App] Not authenticated, showing login')
    authChecked.value = true
    showLogin()
    return
  }
  state.currentUser = user
  isAuthenticated.value = true
  authChecked.value = true
  console.log('[App] Authenticated, isAuthenticated=true')

  await nextTick()
  await startApp()

  if (user.must_change_password) {
    setTimeout(() => openPasswordModal(true), 100)
  }
})
</script>

<template>
  <!-- Login-Ansicht (erst nach Auth-Check zeigen, verhindert Flash) -->
  <div v-if="authChecked && !isAuthenticated" class="login-page">
    <div class="login-card">
      <div class="login-logo">&#x1FAB4; Plankton</div>
      <div v-if="loginError" class="login-error">{{ loginError }}</div>
      <form @submit.prevent="handleLogin">
        <label>
          Username
          <input
            v-model="loginUsername"
            type="text"
            autocomplete="username"
            autofocus
          />
        </label>
        <label>
          Passwort
          <input
            v-model="loginPassword"
            type="password"
            autocomplete="current-password"
          />
        </label>
        <button type="submit" class="btn-primary login-btn">Anmelden</button>
      </form>
    </div>
  </div>
  <!-- Board-Ansicht via AppLayout -->
  <AppLayout v-if="isAuthenticated" :on-logout="showLogin" />
</template>
