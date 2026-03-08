// Allgemeine Hilfsfunktionen.

import { state } from './state.js';

export function escapeHtml(str) {
  return String(str || '')
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;');
}

export function columnName(colId) {
  if (!colId) return '–';
  const col = (state.project?.columns || []).find(c => c.id === colId);
  return col ? col.title : '–';
}

export function formatDate(isoStr) {
  if (!isoStr) return '–';
  try {
    return new Date(isoStr).toLocaleString('de-DE', {
      year: 'numeric', month: '2-digit', day: '2-digit',
      hour: '2-digit', minute: '2-digit',
    });
  } catch { return isoStr; }
}
