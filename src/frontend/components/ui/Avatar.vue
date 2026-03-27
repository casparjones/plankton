<script setup lang="ts">
import { cn } from '@/lib/utils'

const props = withDefaults(defineProps<{
  name?: string
  class?: string
  size?: 'sm' | 'md'
}>(), {
  size: 'sm',
})

const initial = computed(() => props.name?.[0]?.toUpperCase() || '?')

import { computed } from 'vue'
</script>

<template>
  <span :class="cn(
    'inline-flex items-center justify-center rounded-full font-mono uppercase flex-shrink-0',
    props.size === 'sm'
      ? 'w-5 h-5 text-[10px] bg-surface border border-border text-text-dim'
      : 'w-7 h-7 text-xs font-semibold bg-accent-dim border border-accent text-accent',
    props.class
  )" :title="props.name">
    {{ initial }}
  </span>
</template>
