<script setup lang="ts">
import { cn } from '@/lib/utils'

const props = withDefaults(defineProps<{
  class?: string
  danger?: boolean
  disabled?: boolean
}>(), {})

const emit = defineEmits<{
  (e: 'click'): void
}>()
</script>

<template>
  <button
    type="button"
    :disabled="props.disabled"
    :class="cn(
      'w-full text-left flex items-center gap-2 px-3 py-1.5',
      'text-[13px] font-sans cursor-pointer',
      'transition-colors duration-100',
      'disabled:opacity-40 disabled:cursor-not-allowed',
      props.danger
        ? 'text-danger hover:bg-danger/10'
        : 'text-text hover:bg-surface-2',
      props.class
    )"
    @click="!props.disabled && emit('click')"
  >
    <slot />
  </button>
</template>
