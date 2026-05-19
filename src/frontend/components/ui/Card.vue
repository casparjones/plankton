<script setup lang="ts">
import { cn } from '@/lib/utils'

const props = withDefaults(defineProps<{
  class?: string
  variant?: 'default' | 'ghost'
}>(), {
  variant: 'default',
})
</script>

<template>
  <div :class="cn(
    'rounded-lg border border-border bg-surface text-text',
    props.variant === 'ghost' && 'border-transparent bg-transparent',
    props.class
  )">
    <slot />
  </div>
</template>
