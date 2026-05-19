<script setup lang="ts">
import { cn } from '@/lib/utils'
const props = withDefaults(defineProps<{ class?: string }>(), {})
</script>

<template>
  <h2 :class="cn('text-[15px] font-semibold text-text', props.class)">
    <slot />
  </h2>
</template>
