<script setup lang="ts">
import { cn } from '@/lib/utils'
const props = withDefaults(defineProps<{ class?: string }>(), {})
</script>

<template>
  <div :class="cn('flex items-center justify-between px-5 py-4 border-b border-border flex-shrink-0', props.class)">
    <slot />
  </div>
</template>
