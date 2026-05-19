<script setup lang="ts">
import { ref, provide, onMounted, onUnmounted } from 'vue'
import { cn } from '@/lib/utils'

const props = withDefaults(defineProps<{
  class?: string
}>(), {})

const open = ref(false)
provide('dropdown-open', open)

function close() {
  open.value = false
}

function onDocClick(e: MouseEvent) {
  const el = (e.target as HTMLElement).closest('[data-dropdown]')
  if (!el) close()
}

onMounted(() => document.addEventListener('click', onDocClick))
onUnmounted(() => document.removeEventListener('click', onDocClick))
</script>

<template>
  <div data-dropdown class="relative inline-block" :class="props.class">
    <slot :open="open" :toggle="() => (open = !open)" :close="close" />
  </div>
</template>
