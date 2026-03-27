<script setup lang="ts">
import { cn } from '@/lib/utils'

const props = defineProps<{
  class?: string
  bordered?: boolean
}>()
</script>

<template>
  <span :class="cn(
    'font-mono text-[11px] font-semibold uppercase tracking-wider text-text-dim',
    props.bordered && 'border-b border-border pb-1',
    props.class
  )">
    <slot />
  </span>
</template>
