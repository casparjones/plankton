<script setup lang="ts">
import { ref } from 'vue'
import { cn } from '@/lib/utils'

const props = withDefaults(defineProps<{
  text?: string
  side?: 'top' | 'bottom' | 'left' | 'right'
  class?: string
}>(), {
  side: 'top',
})

const visible = ref(false)

const positionClass: Record<string, string> = {
  top:    'bottom-full left-1/2 -translate-x-1/2 mb-1.5',
  bottom: 'top-full left-1/2 -translate-x-1/2 mt-1.5',
  left:   'right-full top-1/2 -translate-y-1/2 mr-1.5',
  right:  'left-full top-1/2 -translate-y-1/2 ml-1.5',
}
</script>

<template>
  <div
    class="relative inline-flex"
    @mouseenter="visible = true"
    @mouseleave="visible = false"
    @focusin="visible = true"
    @focusout="visible = false"
  >
    <slot />
    <Transition name="tooltip">
      <div
        v-if="visible && props.text"
        :class="cn(
          'absolute z-[600] pointer-events-none whitespace-nowrap',
          'px-2 py-1 rounded-md text-[11px] font-sans',
          'bg-surface border border-border text-text',
          'shadow-[0_4px_12px_rgba(0,0,0,0.4)]',
          positionClass[props.side],
          props.class
        )"
        role="tooltip"
      >
        <slot name="content">{{ props.text }}</slot>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.tooltip-enter-active,
.tooltip-leave-active {
  transition: opacity 0.1s ease, transform 0.1s ease;
}
.tooltip-enter-from,
.tooltip-leave-to {
  opacity: 0;
  transform: scale(0.92);
}
</style>
