<script setup lang="ts">
import { cn } from '@/lib/utils'

const props = withDefaults(defineProps<{
  open?: boolean
  variant?: 'default' | 'wide' | 'detail'
  class?: string
}>(), {
  variant: 'default',
})

const emit = defineEmits<{
  (e: 'close'): void
}>()

function onOverlayClick(event: Event) {
  if ((event.target as HTMLElement).classList.contains('dialog-overlay')) {
    emit('close')
  }
}

const sizeClasses = {
  default: 'max-w-[480px]',
  wide: 'max-w-[1000px]',
  detail: 'max-w-[1440px] max-h-[90vh] overflow-y-auto',
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="props.open"
      class="dialog-overlay fixed inset-0 bg-black/70 backdrop-blur-[2px] z-[1000] flex items-center justify-center"
      @click="onOverlayClick"
    >
      <div :class="cn(
        'bg-surface border border-border rounded-lg',
        'shadow-[0_16px_48px_rgba(0,0,0,0.5)]',
        'flex flex-col gap-3.5 p-6 w-[90%]',
        sizeClasses[props.variant],
        props.class
      )">
        <slot />
      </div>
    </div>
  </Teleport>
</template>
