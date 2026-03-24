// Import-Dialog für KI-generierte Tasks.

import api from '../api';
import { state } from '../state';
import { escapeHtml } from '../utils';
import { renderBoard } from './board';
import type { Task, Column, ProjectDoc } from '../types';

export function openImportModal(): void {
  if (!state.project) return;
  (document.getElementById('import-json') as HTMLTextAreaElement).value = '';
  document.getElementById('import-preview')!.innerHTML = '';
  document.getElementById('import-preview')!.style.display = 'none';
  document.getElementById('import-result')!.innerHTML = '';
  document.getElementById('import-result')!.style.display = 'none';
  (document.getElementById('import-start-btn') as HTMLButtonElement).style.display = 'none';
  document.getElementById('import-modal')!.classList.add('open');
  setTimeout(() => (document.getElementById('import-json') as HTMLTextAreaElement).focus(), 50);
}

export function closeImportModal(): void {
  document.getElementById('import-modal')!.classList.remove('open');
}

export function validateImport(): void {
  const text = (document.getElementById('import-json') as HTMLTextAreaElement).value.trim();
  if (!text) return;

  let tasks: Partial<Task>[];
  try {
    const parsed = JSON.parse(text);
    if (!Array.isArray(parsed)) {
      tasks = [parsed];
    } else {
      tasks = parsed;
    }
  } catch {
    document.getElementById('import-preview')!.innerHTML =
      '<div class="import-error">Ungültiges JSON</div>';
    document.getElementById('import-preview')!.style.display = '';
    return;
  }

  const columns: Column[] = state.project!.columns || [];
  const slugs = columns.map(c => (c.slug || '').toUpperCase());

  const rows = tasks.map((t, i) => {
    const idx = i + 1;
    const warnings: string[] = [];
    const errors: string[] = [];

    if (!t.title || !t.title.trim()) {
      errors.push('title fehlt');
    }

    if (t.points !== undefined && (t.points < 0 || t.points > 100)) {
      errors.push(`points ${t.points} außerhalb 0-100`);
    }

    if (t.column_slug) {
      if (!slugs.includes(t.column_slug.toUpperCase())) {
        warnings.push(`column_slug "${t.column_slug}" unbekannt → TODO`);
      }
    } else if (t.column_id) {
      const knownIds = columns.map(c => c.id);
      if (!knownIds.includes(t.column_id)) {
        warnings.push(`column_id unbekannt → TODO`);
      }
    } else {
      warnings.push('Spalte: TODO (Standard)');
    }

    if (!t.creator) warnings.push('creator: wird auto-gesetzt');

    const status = errors.length > 0 ? 'error' : warnings.length > 0 ? 'warning' : 'ok';
    const icon = status === 'error' ? '&#10060;' : status === 'warning' ? '&#9888;' : '&#9989;';
    const notes = [...errors, ...warnings].join('; ') || 'OK';

    return `<tr class="import-row-${status}">
      <td>${idx}</td>
      <td>${icon}</td>
      <td>${escapeHtml(t.title || '(kein Titel)')}</td>
      <td>${escapeHtml(t.column_slug || t.column_id || 'TODO')}</td>
      <td>${t.points || 0}</td>
      <td class="import-notes">${notes}</td>
    </tr>`;
  });

  const validCount = tasks.filter((t) => {
    if (!t.title || !t.title.trim()) return false;
    if (t.points !== undefined && (t.points < 0 || t.points > 100)) return false;
    return true;
  }).length;

  const errorCount = tasks.length - validCount;

  const html = `
    <div class="import-summary">
      <strong>${validCount}</strong> valide, <strong>${errorCount}</strong> Fehler
    </div>
    <table class="import-table">
      <thead><tr><th>#</th><th></th><th>Titel</th><th>Spalte</th><th>Points</th><th>Hinweise</th></tr></thead>
      <tbody>${rows.join('')}</tbody>
    </table>
  `;

  document.getElementById('import-preview')!.innerHTML = html;
  document.getElementById('import-preview')!.style.display = '';

  const btn = document.getElementById('import-start-btn') as HTMLButtonElement;
  if (validCount > 0) {
    btn.style.display = '';
    btn.disabled = false;
  } else {
    btn.style.display = 'none';
  }
}

interface ImportResult {
  imported: number;
  skipped: number;
  warnings: string[];
  errors: string[];
}

export async function executeImport(): Promise<void> {
  const text = (document.getElementById('import-json') as HTMLTextAreaElement).value.trim();
  if (!text) return;

  let tasks: Partial<Task>[];
  try {
    const parsed = JSON.parse(text);
    if (!Array.isArray(parsed)) tasks = [parsed];
    else tasks = parsed;
  } catch { return; }

  // Validierung zuklappen, Button deaktivieren
  document.getElementById('import-preview')!.style.display = 'none';
  const startBtn = document.getElementById('import-start-btn') as HTMLButtonElement;
  startBtn.disabled = true;
  startBtn.textContent = 'Importiere…';

  try {
    const result = await api.post<ImportResult>(`/api/projects/${state.project!._id}/import`, { tasks });
    const resultEl = document.getElementById('import-result')!;
    resultEl.innerHTML = `
      <div class="import-result-summary">
        <strong>${result.imported}</strong> importiert,
        <strong>${result.skipped}</strong> übersprungen
      </div>
      ${result.warnings.length > 0 ? `<div class="import-log"><div class="import-warnings">${result.warnings.map(w => `<div>&#9888; ${escapeHtml(w)}</div>`).join('')}</div></div>` : ''}
      ${result.errors.length > 0 ? `<div class="import-log"><div class="import-errors">${result.errors.map(e => `<div>&#10060; ${escapeHtml(e)}</div>`).join('')}</div></div>` : ''}
    `;
    resultEl.style.display = '';

    if (result.imported > 0) {
      // Alte Task-IDs merken um neue zu erkennen
      const oldIds = new Set((state.project?.tasks || []).map(t => t.id));
      state.project = await api.get<ProjectDoc>(`/api/projects/${state.project!._id}`);
      // Neue Task-IDs für Glow sammeln
      const newIds = (state.project?.tasks || []).filter(t => !oldIds.has(t.id)).map(t => t.id);
      if (newIds.length > 0) {
        (window as any).__newTaskGlowIds = newIds;
      }
      renderBoard();
    }
  } catch (err: any) {
    document.getElementById('import-result')!.innerHTML =
      `<div class="import-error">Fehler: ${escapeHtml(err.message)}</div>`;
    document.getElementById('import-result')!.style.display = '';
  }

  startBtn.style.display = 'none';
  startBtn.textContent = 'Importieren';
}
