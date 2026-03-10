// Composable für Dark/Light Mode Verwaltung.

import { ref } from 'vue'

type Theme = 'dark' | 'light'

const currentTheme = ref<Theme>(getInitialTheme())

/** Ermittelt das gespeicherte oder bevorzugte Theme. */
function getInitialTheme(): Theme {
  const stored = localStorage.getItem('plankton-theme')
  if (stored === 'light' || stored === 'dark') return stored
  return window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark'
}

/** Wendet das Theme auf body und localStorage an. */
function applyTheme(theme: Theme): void {
  document.body.setAttribute('data-theme', theme)
  localStorage.setItem('plankton-theme', theme)
  currentTheme.value = theme
}

/** Composable für Theme-Verwaltung. */
export function useTheme() {
  /** Initialisiert das Theme beim App-Start. */
  function initTheme(): void {
    applyTheme(currentTheme.value)
  }

  /** Wechselt zwischen Dark und Light Mode. */
  function toggleTheme(): void {
    applyTheme(currentTheme.value === 'dark' ? 'light' : 'dark')
  }

  /** Symbol für den Toggle-Button. */
  function themeIcon(): string {
    return currentTheme.value === 'dark' ? '\u2600' : '\u263E'
  }

  return {
    currentTheme,
    initTheme,
    toggleTheme,
    applyTheme,
    themeIcon,
  }
}
