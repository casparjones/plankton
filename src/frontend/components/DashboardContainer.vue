<script setup lang="ts">
// DashboardContainer.vue
// Container-Komponente oberhalb der Kanban-Spalten für Board-Metriken (Counts, Velocity, Burndown).
// Unterstützt Toggle collapsed/expanded mit persistenter localStorage-Preference.
// Mobile: standardmäßig collapsed. Desktop: standardmäßig expanded.

import { ref, computed, onMounted, watch } from 'vue'
import { ChevronDown, ChevronUp, BarChart2 } from 'lucide-vue-next'

// ─── localStorage-Persistenz ─────────────────────────────────────────────────

const STORAGE_KEY = 'plankton_dashboard_visible'

/** Liest den gespeicherten Wert aus localStorage.
 *  Gibt null zurück wenn kein Wert gesetzt ist (dann wird der Geräte-Default genutzt). */
function readStoredVisibility(): boolean | null {
  try {
    const val = localStorage.getItem(STORAGE_KEY)
    if (val === 'true') return true
    if (val === 'false') return false
  } catch {
    // localStorage nicht verfügbar (SSR, Privacy-Mode etc.)
  }
  return null
}

/** Speichert den aktuellen Sichtbarkeitszustand in localStorage. */
function writeStoredVisibility(visible: boolean): void {
  try {
    localStorage.setItem(STORAGE_KEY, visible ? 'true' : 'false')
  } catch {
    // ignore
  }
}

/** Gibt den Platform-Default zurück: Mobile = collapsed (false), Desktop = expanded (true). */
function platformDefault(): boolean {
  if (typeof window === 'undefined') return true
  return window.innerWidth >= 768
}

// ─── State ───────────────────────────────────────────────────────────────────

const isExpanded = ref<boolean>(true)

onMounted(() => {
  const stored = readStoredVisibility()
  isExpanded.value = stored !== null ? stored : platformDefault()
})

/** Toggle-Handler: wechselt collapsed/expanded und speichert Preference. */
function toggle(): void {
  isExpanded.value = !isExpanded.value
  writeStoredVisibility(isExpanded.value)
}

// Watcher für programmatisches Ändern (falls von außen gesetzt)
watch(isExpanded, (val) => {
  writeStoredVisibility(val)
})
</script>

<template>
  <div
    data-testid="dashboard-container"
    :data-expanded="String(isExpanded)"
    class="dashboard-container border-b border-border bg-surface transition-all duration-200"
  >
    <!-- Header-Leiste mit Toggle-Button -->
    <div
      class="flex items-center gap-2 px-6 py-2 cursor-pointer select-none hover:bg-surface-2 transition-colors duration-150"
      @click="toggle"
    >
      <BarChart2 class="w-3.5 h-3.5 text-text-dim flex-shrink-0" />
      <span class="font-mono text-[11px] font-semibold uppercase tracking-wider text-text-dim">
        Dashboard
      </span>
      <button
        data-testid="dashboard-toggle"
        class="ml-auto bg-transparent border-none p-0 cursor-pointer text-text-dim hover:text-accent transition-colors"
        :title="isExpanded ? 'Dashboard ausblenden' : 'Dashboard einblenden'"
        @click.stop="toggle"
        aria-label="Dashboard umschalten"
      >
        <ChevronUp v-if="isExpanded" class="w-4 h-4" />
        <ChevronDown v-else class="w-4 h-4" />
      </button>
    </div>

    <!-- Widget-Bereich (nur sichtbar wenn expanded) -->
    <div
      v-show="isExpanded"
      data-testid="dashboard-widgets"
      class="flex gap-3 px-6 pb-3 flex-wrap"
    >
      <!-- Slot: Counts (Task-Anzahl pro Spalte) -->
      <div
        data-testid="dashboard-widget-slot"
        class="dashboard-widget-slot flex-1 min-w-[140px] bg-surface-2 border border-border rounded-md p-2.5"
      >
        <slot name="counts">
          <span class="font-mono text-[10px] text-text-dim uppercase tracking-wide">Counts</span>
        </slot>
      </div>

      <!-- Slot: Velocity (Story-Points pro Zeitraum) -->
      <div
        data-testid="dashboard-widget-slot"
        class="dashboard-widget-slot flex-1 min-w-[140px] bg-surface-2 border border-border rounded-md p-2.5"
      >
        <slot name="velocity">
          <span class="font-mono text-[10px] text-text-dim uppercase tracking-wide">Velocity</span>
        </slot>
      </div>

      <!-- Slot: Burndown-Chart -->
      <div
        data-testid="dashboard-widget-slot"
        class="dashboard-widget-slot flex-1 min-w-[140px] bg-surface-2 border border-border rounded-md p-2.5"
      >
        <slot name="burndown">
          <span class="font-mono text-[10px] text-text-dim uppercase tracking-wide">Burndown</span>
        </slot>
      </div>
    </div>
  </div>
</template>

<style scoped>
.dashboard-container {
  /* Kein margin/padding das Layout-Shift verursacht wenn collapsed */
}
</style>
