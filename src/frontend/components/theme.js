// Theme (Dark/Light Mode)

export function applyTheme(theme) {
  document.body.setAttribute('data-theme', theme);
  localStorage.setItem('plankton-theme', theme);
  const toggle = document.getElementById('theme-toggle');
  if (toggle) toggle.textContent = theme === 'dark' ? '\u2600' : '\u263E';
}

export function toggleTheme() {
  const current = document.body.getAttribute('data-theme') || 'dark';
  applyTheme(current === 'dark' ? 'light' : 'dark');
}

export function initTheme() {
  const stored = localStorage.getItem('plankton-theme');
  if (stored) {
    applyTheme(stored);
  } else {
    const prefersLight = window.matchMedia('(prefers-color-scheme: light)').matches;
    applyTheme(prefersLight ? 'light' : 'dark');
  }
}
