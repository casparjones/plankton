// Theme (Dark/Light Mode)

export function applyTheme(theme: string): void {
  document.body.setAttribute('data-theme', theme);
  localStorage.setItem('plankton-theme', theme);
  const toggle = document.getElementById('theme-toggle');
  if (toggle) toggle.textContent = theme === 'dark' ? '☀' : '☾';
}

export function toggleTheme(): void {
  const current = document.body.getAttribute('data-theme') || 'dark';
  applyTheme(current === 'dark' ? 'light' : 'dark');
}

export function initTheme(): void {
  const stored = localStorage.getItem('plankton-theme');
  if (stored) {
    applyTheme(stored);
  } else {
    const prefersLight = window.matchMedia('(prefers-color-scheme: light)').matches;
    applyTheme(prefersLight ? 'light' : 'dark');
  }
}
