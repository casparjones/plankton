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

/** Label-Farbe basierend auf Inhalt. */
export function labelColor(label: string): { bg: string; border: string; color: string } {
  const l = label.toLowerCase()
  const map: Record<string, { bg: string; border: string; color: string }> = {
    bug:         { bg: '#3a1c1c', border: '#e53935', color: '#ff8a80' },
    fix:         { bg: '#3a1c1c', border: '#e53935', color: '#ff8a80' },
    feature:     { bg: '#1a2e1a', border: '#43a047', color: '#a5d6a7' },
    enhancement: { bg: '#1a2a3a', border: '#1e88e5', color: '#90caf9' },
    review:      { bg: '#3a2e1a', border: '#fb8c00', color: '#ffcc80' },
    design:      { bg: '#2a1a3a', border: '#8e24aa', color: '#ce93d8' },
    docs:        { bg: '#1a3a3a', border: '#00897b', color: '#80cbc4' },
    test:        { bg: '#3a3a1a', border: '#c0ca33', color: '#e6ee9c' },
    testing:     { bg: '#3a3a1a', border: '#c0ca33', color: '#e6ee9c' },
    ui:          { bg: '#2a1a3a', border: '#8e24aa', color: '#ce93d8' },
    mcp:         { bg: '#1a2a3a', border: '#1e88e5', color: '#90caf9' },
    cli:         { bg: '#1a3a2a', border: '#26a69a', color: '#80cbc4' },
    icon:        { bg: '#2a1a3a', border: '#8e24aa', color: '#ce93d8' },
    refactor:    { bg: '#2a2a1a', border: '#fdd835', color: '#fff59d' },
    security:    { bg: '#3a1c1c', border: '#e53935', color: '#ff8a80' },
    performance: { bg: '#1a2a3a', border: '#1e88e5', color: '#90caf9' },
  }
  if (map[l]) return map[l]
  let hash = 0
  for (let i = 0; i < label.length; i++) hash = label.charCodeAt(i) + ((hash << 5) - hash)
  const hue = Math.abs(hash) % 360
  return { bg: `hsl(${hue}, 30%, 15%)`, border: `hsl(${hue}, 60%, 50%)`, color: `hsl(${hue}, 70%, 75%)` }
}
