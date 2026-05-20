<script setup lang="ts">
// BurndownWidget.vue
// Widget für den DashboardContainer-Slot #burndown.
// Zeigt täglichen Burndown (verbleibende Tasks oder Points) als SVG-Line-Chart
// mit gestrichelter Ideal-Linie an.
//
// HINWEIS: Integration in AppLayout.vue ist absichtlich ausgespart –
// AppLayout.vue wird gerade vom Tester geprüft. Integration erfolgt
// separat nach Freigabe durch den Tester (siehe Ticket-Kommentar).

import { ref, computed, watch } from 'vue'
import { RefreshCw } from 'lucide-vue-next'
import api from '../api'
import { state } from '../state'

// ─── Typen ───────────────────────────────────────────────────────────────────

interface BurndownEntry {
  date: string
  remaining_tasks: number
  remaining_points: number
  ideal_tasks: number
  ideal_points: number
}

type RangePreset = '7d' | '30d' | '90d'
type ToggleMode = 'tasks' | 'points'

// ─── State ───────────────────────────────────────────────────────────────────

const entries = ref<BurndownEntry[]>([])
const loading = ref(false)
const error = ref<string | null>(null)
const rangePreset = ref<RangePreset>('30d')
const mode = ref<ToggleMode>('tasks')

// ─── Hilfsfunktionen: Datum-Berechnung ──────────────────────────────────────

/** Gibt ein ISO-Datum (YYYY-MM-DD) für `offsetDays` Tage vor heute zurück. */
function isoDate(offsetDays: number): string {
  const d = new Date()
  d.setDate(d.getDate() - offsetDays)
  return d.toISOString().slice(0, 10)
}

/** Gibt die Anzahl Tage für ein RangePreset zurück. */
function presetDays(preset: RangePreset): number {
  return preset === '7d' ? 6 : preset === '30d' ? 29 : 89
}

// ─── Daten laden ─────────────────────────────────────────────────────────────

async function fetchBurndown(): Promise<void> {
  const projectId = state.project?._id
  if (!projectId) return
  loading.value = true
  error.value = null
  try {
    const days = presetDays(rangePreset.value)
    const from = isoDate(days)
    const to = isoDate(0)
    entries.value = await api.get<BurndownEntry[]>(
      `/api/projects/${projectId}/stats/burndown?from=${from}&to=${to}`
    )
  } catch (e) {
    error.value = e instanceof Error ? e.message : 'Fehler beim Laden'
  } finally {
    loading.value = false
  }
}

// Initialer Load + Projekt-Wechsel
watch(
  () => state.project?._id,
  (id) => {
    if (id) fetchBurndown()
  },
  { immediate: true }
)

// Bei Range-Wechsel neu laden
watch(rangePreset, () => {
  if (state.project?._id) fetchBurndown()
})

// ─── SVG-Linechart ───────────────────────────────────────────────────────────

const SVG_W = 180
const SVG_H = 48
const PADDING_LEFT = 4
const PADDING_RIGHT = 4
const PADDING_TOP = 4
const PADDING_BOTTOM = 4

/** Aktueller Wert (remaining) je nach Mode */
const remainingKey = computed<'remaining_tasks' | 'remaining_points'>(() =>
  mode.value === 'tasks' ? 'remaining_tasks' : 'remaining_points'
)

/** Ideal-Wert je nach Mode */
const idealKey = computed<'ideal_tasks' | 'ideal_points'>(() =>
  mode.value === 'tasks' ? 'ideal_tasks' : 'ideal_points'
)

/** Maximaler Wert für Y-Skalierung (entweder ideal oder actual, whichever is higher) */
const yMax = computed<number>(() => {
  if (entries.value.length === 0) return 1
  const maxActual = Math.max(...entries.value.map((e) => e[remainingKey.value]), 0)
  const maxIdeal = Math.max(...entries.value.map((e) => e[idealKey.value]), 0)
  return Math.max(maxActual, maxIdeal, 1)
})

/** Konvertiert Datenpunkte in SVG-Koordinaten */
function toPoints(values: number[]): string {
  if (values.length === 0) return ''
  const n = values.length
  const chartW = SVG_W - PADDING_LEFT - PADDING_RIGHT
  const chartH = SVG_H - PADDING_TOP - PADDING_BOTTOM
  return values
    .map((v, i) => {
      const x = PADDING_LEFT + (i / Math.max(n - 1, 1)) * chartW
      const y = PADDING_TOP + (1 - v / yMax.value) * chartH
      return `${x.toFixed(1)},${y.toFixed(1)}`
    })
    .join(' ')
}

/** Polyline-Punkte für den tatsächlichen Burndown */
const actualPoints = computed<string>(() =>
  toPoints(entries.value.map((e) => e[remainingKey.value]))
)

/** Polyline-Punkte für die Ideal-Linie */
const idealPoints = computed<string>(() =>
  toPoints(entries.value.map((e) => e[idealKey.value]))
)

// ─── Aktueller Restbestand (letzter Datenpunkt) ───────────────────────────────

const currentRemaining = computed<number>(() => {
  if (entries.value.length === 0) return 0
  return entries.value[entries.value.length - 1][remainingKey.value]
})

const startTotal = computed<number>(() => {
  if (entries.value.length === 0) return 0
  return entries.value[0][remainingKey.value]
})

// ─── Label für Toggle ─────────────────────────────────────────────────────────

const modeLabel = computed(() => (mode.value === 'tasks' ? 'Tasks' : 'Points'))
</script>

<template>
  <div data-testid="burndown-widget" class="w-full">
    <!-- Header: Titel + Range-Buttons + Toggle + Refresh -->
    <div class="flex items-center justify-between mb-1.5 gap-1 flex-wrap">
      <span class="font-mono text-[10px] font-semibold uppercase tracking-wider text-text-dim">
        Burndown
      </span>

      <div class="flex items-center gap-1 ml-auto">
        <!-- Range-Buttons -->
        <div class="flex gap-0.5">
          <button
            v-for="preset in (['7d', '30d', '90d'] as const)"
            :key="preset"
            class="font-mono text-[9px] px-1 py-0.5 rounded border transition-colors"
            :class="
              rangePreset === preset
                ? 'border-accent text-accent bg-accent/10'
                : 'border-border text-text-dim hover:text-text hover:border-text-dim'
            "
            :data-testid="`burndown-range-${preset}`"
            @click="rangePreset = preset"
          >
            {{ preset }}
          </button>
        </div>

        <!-- Tasks vs. Points Toggle -->
        <button
          class="font-mono text-[9px] px-1 py-0.5 rounded border border-border text-text-dim hover:text-accent hover:border-accent transition-colors"
          data-testid="burndown-mode-toggle"
          :title="`Anzeige: ${modeLabel} – klicken zum Wechseln`"
          @click="mode = mode === 'tasks' ? 'points' : 'tasks'"
        >
          {{ modeLabel }}
        </button>

        <!-- Refresh -->
        <button
          class="bg-transparent border-none p-0 cursor-pointer text-text-dim hover:text-accent transition-colors disabled:opacity-40"
          data-testid="burndown-refresh"
          :disabled="loading"
          title="Burndown aktualisieren"
          @click="fetchBurndown"
        >
          <RefreshCw class="w-3 h-3" :class="{ 'animate-spin': loading }" />
        </button>
      </div>
    </div>

    <!-- Fehler -->
    <div
      v-if="error"
      class="font-mono text-[10px] text-red-400"
      data-testid="burndown-error"
    >
      {{ error }}
    </div>

    <!-- Daten vorhanden -->
    <template v-else-if="entries.length">
      <!-- Kennzahl: verbleibend -->
      <div class="flex items-end gap-3 mb-1.5">
        <div class="flex flex-col">
          <span class="font-mono text-[18px] font-bold text-accent leading-none">
            {{ currentRemaining }}
          </span>
          <span class="font-mono text-[9px] text-text-dim uppercase tracking-wide mt-0.5">
            verbleibend
          </span>
        </div>
        <div class="flex flex-col">
          <span class="font-mono text-[13px] font-semibold text-text leading-none">
            {{ startTotal }}
          </span>
          <span class="font-mono text-[9px] text-text-dim uppercase tracking-wide mt-0.5">
            start
          </span>
        </div>
      </div>

      <!-- SVG Line-Chart -->
      <svg
        :width="SVG_W"
        :height="SVG_H"
        class="block overflow-visible"
        data-testid="burndown-chart"
        aria-hidden="true"
      >
        <!-- Ideal-Linie (gestrichelt) -->
        <polyline
          v-if="idealPoints"
          :points="idealPoints"
          fill="none"
          stroke="var(--color-text-dim, #666)"
          stroke-width="1"
          stroke-dasharray="3,3"
          opacity="0.5"
        />
        <!-- Tatsächlicher Burndown -->
        <polyline
          v-if="actualPoints"
          :points="actualPoints"
          fill="none"
          stroke="var(--color-accent)"
          stroke-width="1.5"
          stroke-linejoin="round"
          stroke-linecap="round"
        />
      </svg>

      <!-- Legende -->
      <div class="flex gap-3 mt-1">
        <div class="flex items-center gap-1">
          <svg width="12" height="4" aria-hidden="true">
            <line
              x1="0" y1="2" x2="12" y2="2"
              stroke="var(--color-accent)"
              stroke-width="1.5"
            />
          </svg>
          <span class="font-mono text-[8px] text-text-dim">Aktuell</span>
        </div>
        <div class="flex items-center gap-1">
          <svg width="12" height="4" aria-hidden="true">
            <line
              x1="0" y1="2" x2="12" y2="2"
              stroke="var(--color-text-dim, #666)"
              stroke-width="1"
              stroke-dasharray="3,3"
              opacity="0.5"
            />
          </svg>
          <span class="font-mono text-[8px] text-text-dim">Ideal</span>
        </div>
      </div>
    </template>

    <!-- Lade-Zustand -->
    <div
      v-else-if="loading"
      class="font-mono text-[10px] text-text-dim"
      data-testid="burndown-loading"
    >
      …
    </div>

    <!-- Leer-Zustand -->
    <div
      v-else
      class="font-mono text-[10px] text-text-dim"
      data-testid="burndown-empty"
    >
      Keine Daten
    </div>
  </div>
</template>
