<script setup lang="ts">
// StatusCountsWidget.vue
// Widget für den DashboardContainer-Slot #counts.
// Zeigt die Anzahl Tasks pro sichtbarer Spalte an.
// Unterstützt manuellen Refresh sowie automatisches Update via state.project.

import { ref, computed, watch } from 'vue'
import { RefreshCw } from 'lucide-vue-next'
import api from '../api'
import { state } from '../state'

// ─── Typen ───────────────────────────────────────────────────────────────────

interface ColumnStat {
  column_id: string
  title: string
  task_count: number
}

// ─── State ───────────────────────────────────────────────────────────────────

const stats = ref<ColumnStat[]>([])
const loading = ref(false)
const error = ref<string | null>(null)

// ─── Daten laden ─────────────────────────────────────────────────────────────

async function fetchStats(): Promise<void> {
  const projectId = state.project?._id
  if (!projectId) return
  loading.value = true
  error.value = null
  try {
    stats.value = await api.get<ColumnStat[]>(`/api/projects/${projectId}/stats/columns`)
  } catch (e) {
    error.value = e instanceof Error ? e.message : 'Fehler beim Laden'
  } finally {
    loading.value = false
  }
}

// Initialer Load und bei Projekt-Wechsel
watch(
  () => state.project?._id,
  (id) => {
    if (id) fetchStats()
  },
  { immediate: true }
)

// Bei Board-Änderungen (Task-Moves, neue Tasks) Stats aktualisieren
watch(
  () => state.project?.tasks?.length,
  () => {
    if (state.project?._id) fetchStats()
  }
)

// ─── Computed: Spaltenfarbe aus project.columns ───────────────────────────────

function columnColor(columnId: string): string {
  const col = state.project?.columns.find((c) => c.id === columnId)
  return col?.color ?? 'var(--color-accent)'
}

// ─── Computed: Gesamt-Taskanzahl ─────────────────────────────────────────────

const totalCount = computed<number>(() =>
  stats.value.reduce((sum, s) => sum + s.task_count, 0)
)
</script>

<template>
  <div data-testid="status-counts-widget" class="w-full">
    <!-- Widget-Header -->
    <div class="flex items-center justify-between mb-2">
      <span class="font-mono text-[10px] font-semibold uppercase tracking-wider text-text-dim">
        Tasks / Spalte
      </span>
      <button
        data-testid="status-counts-refresh"
        class="bg-transparent border-none p-0 cursor-pointer text-text-dim hover:text-accent transition-colors disabled:opacity-40"
        :disabled="loading"
        title="Counts aktualisieren"
        @click="fetchStats"
      >
        <RefreshCw
          class="w-3 h-3"
          :class="{ 'animate-spin': loading }"
        />
      </button>
    </div>

    <!-- Fehler-Meldung -->
    <div
      v-if="error"
      class="font-mono text-[10px] text-red-400"
      data-testid="status-counts-error"
    >
      {{ error }}
    </div>

    <!-- Stats-Liste -->
    <div
      v-else-if="stats.length"
      class="flex flex-col gap-1"
      data-testid="status-counts-list"
    >
      <div
        v-for="stat in stats"
        :key="stat.column_id"
        class="flex items-center gap-1.5"
        :data-column-id="stat.column_id"
      >
        <!-- Farbpunkt passend zur Spaltenfarbe -->
        <span
          class="inline-block w-2 h-2 rounded-full flex-shrink-0"
          :style="{ background: columnColor(stat.column_id) }"
        />
        <!-- Spaltenname -->
        <span class="font-mono text-[11px] text-text-dim flex-1 truncate">
          {{ stat.title }}
        </span>
        <!-- Task-Count Badge -->
        <span
          class="bg-surface border border-border rounded-[10px] font-mono text-[10px] text-text-dim px-[6px] py-px flex-shrink-0"
          :data-testid="`count-${stat.column_id}`"
        >
          {{ stat.task_count }}
        </span>
      </div>
      <!-- Gesamtzeile -->
      <div class="flex items-center gap-1.5 border-t border-border pt-1 mt-0.5">
        <span class="inline-block w-2 h-2 flex-shrink-0" />
        <span class="font-mono text-[11px] text-text-dim flex-1">Gesamt</span>
        <span
          class="bg-accent-dim border border-accent rounded-[10px] font-mono text-[10px] text-accent px-[6px] py-px flex-shrink-0"
          data-testid="status-counts-total"
        >
          {{ totalCount }}
        </span>
      </div>
    </div>

    <!-- Lade-Spinner (initial, keine Stats vorhanden) -->
    <div
      v-else-if="loading"
      class="font-mono text-[10px] text-text-dim"
    >
      …
    </div>

    <!-- Leer-Zustand -->
    <div
      v-else
      class="font-mono text-[10px] text-text-dim"
      data-testid="status-counts-empty"
    >
      Keine Spalten
    </div>
  </div>
</template>
