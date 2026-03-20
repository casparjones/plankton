// Zentraler Anwendungs-State und Konstanten.
// Nutzt Vue reactive() damit Vue-Komponenten Änderungen tracken können.

import { reactive } from 'vue';
import type { AppState } from './types';

export const state: AppState = reactive({
  projects: [],
  project: null,
  kanban: null,
  editingTask: null,
  isNewTask: false,
  selectedTasks: new Set<string>(),
  eventSource: null,
  currentUser: null,
  isDragging: false,
  detailTask: null,
  allUsers: [],
});

// 20 vordefinierte Farben für Spalten.
export const COLUMN_COLORS: string[] = [
  '#90CAF9', '#FFCC80', '#A5D6A7', '#EF9A9A', '#CE93D8',
  '#80DEEA', '#FFF59D', '#FFAB91', '#B0BEC5', '#F48FB1',
  '#81D4FA', '#C5E1A5', '#BCAAA4', '#B39DDB', '#80CBC4',
  '#FFE082', '#9FA8DA', '#E6EE9C', '#FFCCBC', '#D1C4E9',
];
