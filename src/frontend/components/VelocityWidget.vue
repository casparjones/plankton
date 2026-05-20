<script setup lang="ts">
// VelocityWidget.vue
// Widget für den DashboardContainer-Slot #velocity.
// Zeigt wöchentliche Velocity (erledigte Story-Points) als Sparkline (SVG-Bars) +
// Durchschnitt der letzten 4 Wochen an.

import { ref, computed, watch } from 'vue'
import { RefreshCw } from 'lucide-vue-next'
import api from '../api'
import { state } from '../state'

// ─── Typen ───────────────────────────────────────────────────────────────────

interface VelocityEntry {
  week_start: string
  points_done: number
  tasks_done: number
}

// ─── State ───────────────────────────────────────────────────────────────────

const entries = ref<VelocityEntry[]>([])
const loading = ref(false)
const error = ref<string | null>(null)

// ─── Daten laden ─────────────────────────────────────────────────────────────

async function fetchVelocity(): Promise<void> {
  const projectId = state.project?._id
  if (!projectId) return
  loading.value = true
  error.value = null
  try {
    entries.value = await api.get<VelocityEntry[]>(
      `/api/projects/${projectId}/stats/velocity?weeks=8`
    )
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
    if (id) fetchVelocity()
  },
  { immediate: true }
)

// Bei Board-Änderungen (Task-Moves) Velocity aktualisieren
watch(
  () => state.project?.tasks?.length,
  () => {
    if (state.project?._id) fetchVelocity()
  }
)

// ─── Computed: Durchschnitt der letzten 4 Wochen ─────────────────────────────

const avg4 = computed<number>(() => {
  const last4 = entries.value.slice(-4)
  if (last4.length === 0) return 0
  const sum = last4.reduce((acc, e) => acc + e.points_done, 0)
  return Math.round(sum / last4.length)
})

// ─── Computed: Sparkline SVG-Bars ────────────────────────────────────────────

const SVG_W = 120
const SVG_H = 28
const BAR_GAP = 2

const sparklineBars = computed(() => {
  if (entries.value.length === 0) return []
  const maxPoints = Math.max(...entries.value.map((e) => e.points_done), 1)
  const n = entries.value.length
  const barW = Math.max(1, (SVG_W - BAR_GAP * (n - 1)) / n)

  return entries.value.map((e, i) => {
    const barH = Math.max(2, (e.points_done / maxPoints) * (SVG_H - 4))
    return {
      x: i * (barW + BAR_GAP),
      y: SVG_H - barH,
      width: barW,
      height: barH,
      points: e.points_done,
      tasks: e.tasks_done,
      week: e.week_start,
      // Letzte Woche hervorheben
      current: i === entries.value.length - 1,
    }
  })
})

// ─── Computed: Letzte Woche Points ───────────────────────────────────────────

const lastWeekPoints = computed<number>(() => {
  if (entries.value.length === 0) return 0
  return entries.value[entries.value.length - 1].points_done
})
</script>

<template>
  <div data-testid="velocity-widget" class="w-full">
    <!-- Widget-Header -->
    <div class="flex items-center justify-between mb-2">
      <span class="font-mono text-[10px] font-semibold uppercase tracking-wider text-text-dim">
        Velocity
      </span>
      <button
        data-testid="velocity-refresh"
        class="bg-transparent border-none p-0 cursor-pointer text-text-dim hover:text-accent transition-colors disabled:opacity-40"
        :disabled="loading"
        title="Velocity aktualisieren"
        @click="fetchVelocity"
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
      data-testid="velocity-error"
    >
      {{ error }}
    </div>

    <!-- Daten vorhanden -->
    <template v-else-if="entries.length">
      <!-- Kennzahlen-Zeile: Ø + letzte Woche -->
      <div class="flex items-end gap-3 mb-2">
        <!-- Durchschnitt letzte 4 Wochen -->
        <div class="flex flex-col">
          <span class="font-mono text-[18px] font-bold text-accent leading-none">
            {{ avg4 }}
          </span>
          <span class="font-mono text-[9px] text-text-dim uppercase tracking-wide mt-0.5">
            &#216; 4W pts
          </span>
        </div>
        <!-- Letzte Woche -->
        <div class="flex flex-col">
          <span class="font-mono text-[13px] font-semibold text-text leading-none">
            {{ lastWeekPoints }}
          </span>
          <span class="font-mono text-[9px] text-text-dim uppercase tracking-wide mt-0.5">
            diese Woche
          </span>
        </div>
      </div>

      <!-- Sparkline (SVG-Bars) -->
      <svg
        :width="SVG_W"
        :height="SVG_H"
        class="block overflow-visible"
        data-testid="velocity-sparkline"
        aria-hidden="true"
      >
        <rect
          v-for="bar in sparklineBars"
          :key="bar.week"
          :x="bar.x"
          :y="bar.y"
          :width="bar.width"
          :height="bar.height"
          :fill="bar.current ? 'var(--color-accent)' : 'var(--color-accent-dim, #4a6fa5)'"
          rx="1"
          :opacity="bar.current ? 1 : 0.55"
        >
          <title>{{ bar.week }}: {{ bar.points }} pts, {{ bar.tasks }} Tasks</title>
        </rect>
      </svg>
    </template>

    <!-- Lade-Zustand -->
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
      data-testid="velocity-empty"
    >
      Keine Daten
    </div>
  </div>
</template>
