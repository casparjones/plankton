<script setup lang="ts">
defineProps<{
  title: string
}>()

const emit = defineEmits<{
  (e: 'close'): void
}>()
</script>

<template>
  <div class="flex items-center justify-between">
    <span class="font-mono text-[13px] font-semibold tracking-wide uppercase text-text-dim">
      {{ title }}
    </span>
    <button
      class="bg-transparent border-none text-text-dim cursor-pointer text-base px-1.5 py-0.5 hover:text-text transition-colors"
      @click="emit('close')"
    >&#10005;</button>
  </div>
</template>
