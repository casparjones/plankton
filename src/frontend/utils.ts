// Allgemeine Hilfsfunktionen.

import { state } from './state';

export function escapeHtml(str: string): string {
  return String(str || '')
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;');
}

export function columnName(colId: string): string {
  if (!colId) return '–';
  const col = (state.project?.columns || []).find(c => c.id === colId);
  return col ? col.title : '–';
}

export function formatDate(isoStr: string): string {
  if (!isoStr) return '–';
  try {
    return new Date(isoStr).toLocaleString('de-DE', {
      year: 'numeric', month: '2-digit', day: '2-digit',
      hour: '2-digit', minute: '2-digit',
    });
  } catch { return isoStr; }
}
