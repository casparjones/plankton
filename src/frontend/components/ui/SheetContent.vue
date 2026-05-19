<script setup lang="ts">
import { cn } from '@/lib/utils'
const props = withDefaults(defineProps<{ class?: string }>(), {})
</script>

<template>
  <div :class="cn('flex-1 overflow-y-auto p-5', props.class)">
    <slot />
  </div>
</template>
