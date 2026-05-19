<script setup lang="ts">
import { cn } from '@/lib/utils'

const props = withDefaults(defineProps<{
  class?: string
}>(), {})
</script>

<template>
  <div :class="cn('p-4', props.class)">
    <slot />
  </div>
</template>
