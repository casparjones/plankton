<script setup lang="ts">
// Plankton – Root-Komponente.
// Steuert die Hauptansicht: Login-Screen oder Board (via AppLayout).

import { ref, computed, onMounted, nextTick } from 'vue'
import { useTheme } from './composables/useTheme'
import AppLayout from './components/AppLayout.vue'
import ImportPage from './components/ImportPage.vue'
import type { Claims, Task } from './types'

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
const isImportRoute = computed(() => location.pathname === '/import')
const loginError = ref('')
const loginUsername = ref('')
const loginPassword = ref('')

/** Parst die URL und gibt Projekt- und Task-ID zurück. */
function parseRoute(): { projectId?: string; taskId?: string } {
  const match = location.pathname.match(/^\/p\/([^/]+)(?:\/t\/([^/]+))?/)
  if (match) return { projectId: match[1], taskId: match[2] }
  return {}
}

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
  await loadProjects()

  if (state.projects.length > 0) {
    const route = parseRoute()
    // URL-Projekt hat Vorrang, dann localStorage, dann erstes Projekt.
    const target = route.projectId && state.projects.find((p: { _id: string }) => p._id === route.projectId)
      ? route.projectId
      : (() => {
          const lastId = getLastProject()
          return lastId && state.projects.find((p: { _id: string }) => p._id === lastId)
            ? lastId
            : state.projects[0]._id
        })()
    await openProject(target, true)
    // URL setzen falls noch auf / (ohne pushState-Duplikat).
    if (!location.pathname.startsWith('/p/')) {
      history.replaceState({ project: target }, '', `/p/${target}`)
    }
    // Task aus URL öffnen.
    if (route.taskId && state.project) {
      const task = state.project.tasks.find((t: Task) => t.id === route.taskId)
      if (task) {
        await nextTick()
        // @ts-ignore
        window.__openTaskDetail?.(task)
      }
    }
  }

  // Browser-Back/Forward: Projekt und Task synchronisieren.
  window.addEventListener('popstate', async (e) => {
    const s = e.state as { project?: string; task?: string } | null
    if (s?.project && s.project !== state.project?._id) {
      await openProject(s.project, true)
    }
    if (s?.task && state.project) {
      const task = state.project.tasks.find((t: Task) => t.id === s.task)
      if (task) {
        // @ts-ignore
        window.__openTaskDetail?.(task)
      }
    } else {
      // @ts-ignore
      window.__closeTaskDetail?.()
    }
  })
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
  if (location.pathname !== '/import') {
    await startApp()
  }

  if (user.must_change_password) {
    setTimeout(() => openPasswordModal(true), 100)
  }
})
</script>

<template>
  <!-- Login-Ansicht (erst nach Auth-Check zeigen, verhindert Flash) -->
  <div v-if="authChecked && !isAuthenticated" class="login-page">
    <img src="/icons/plankton-splash.png" alt="" class="login-splash" />
    <div class="login-card">
      <div class="login-logo">
        <img src="/icons/logo.svg" alt="Plankton" class="login-logo-img" />
        Plankton
      </div>
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
  <!-- Import-Seite (mobile-optimiert) -->
  <ImportPage v-if="isAuthenticated && isImportRoute" />
  <!-- Board-Ansicht via AppLayout -->
  <AppLayout v-else-if="isAuthenticated" :on-logout="showLogin" />
</template>
