<script setup lang="ts">
import { cn } from '@/lib/utils'

const props = withDefaults(defineProps<{
  class?: string
  orientation?: 'horizontal' | 'vertical'
}>(), {
  orientation: 'horizontal',
})
</script>

<template>
  <div
    :class="cn(
      'bg-border shrink-0',
      props.orientation === 'horizontal' ? 'h-px w-full' : 'w-px h-full',
      props.class
    )"
  />
</template>
