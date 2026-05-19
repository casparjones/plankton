<script setup lang="ts">
import { cn } from '@/lib/utils'
const props = withDefaults(defineProps<{ class?: string }>(), {})
</script>

<template>
  <div :class="cn('h-px bg-border my-1 mx-1', props.class)" />
</template>
