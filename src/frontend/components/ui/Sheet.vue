<script setup lang="ts">
import { cn } from '@/lib/utils'

const props = withDefaults(defineProps<{
  open?: boolean
  side?: 'left' | 'right' | 'top' | 'bottom'
  class?: string
}>(), {
  side: 'right',
})

const emit = defineEmits<{
  (e: 'close'): void
}>()

const sideClasses: Record<string, string> = {
  right: 'right-0 top-0 h-full w-[360px] max-w-[90vw]',
  left:  'left-0 top-0 h-full w-[360px] max-w-[90vw]',
  top:   'top-0 left-0 w-full h-auto',
  bottom:'bottom-0 left-0 w-full h-auto',
}

const slideFrom: Record<string, string> = {
  right:  'translate-x-full',
  left:   '-translate-x-full',
  top:    '-translate-y-full',
  bottom: 'translate-y-full',
}
</script>

<template>
  <Teleport to="body">
    <Transition name="sheet-overlay">
      <div
        v-if="props.open"
        class="fixed inset-0 bg-black/60 backdrop-blur-[2px] z-[900]"
        @click="emit('close')"
      />
    </Transition>
    <Transition name="sheet-panel">
      <div
        v-if="props.open"
        :class="cn(
          'fixed z-[950] bg-surface border-border flex flex-col',
          props.side === 'right' ? 'border-l' : '',
          props.side === 'left'  ? 'border-r' : '',
          props.side === 'top'   ? 'border-b' : '',
          props.side === 'bottom'? 'border-t' : '',
          'shadow-[0_0_40px_rgba(0,0,0,0.5)]',
          sideClasses[props.side],
          props.class
        )"
      >
        <slot />
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.sheet-overlay-enter-active,
.sheet-overlay-leave-active {
  transition: opacity 0.2s ease;
}
.sheet-overlay-enter-from,
.sheet-overlay-leave-to {
  opacity: 0;
}

.sheet-panel-enter-active,
.sheet-panel-leave-active {
  transition: transform 0.25s cubic-bezier(0.32, 0.72, 0, 1);
}
.sheet-panel-enter-from,
.sheet-panel-leave-to {
  transform: v-bind('slideFrom[props.side]');
}
</style>
