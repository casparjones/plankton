<script setup lang="ts">
import { cn } from '@/lib/utils'

const props = withDefaults(defineProps<{
  class?: string
  modelValue?: string
  rows?: number
}>(), {
  rows: 4,
})

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
}>()
</script>

<template>
  <textarea
    :rows="props.rows"
    :value="props.modelValue"
    :class="cn(
      'w-full bg-surface-2 border border-border rounded-md text-text text-sm',
      'px-3 py-2 font-sans outline-none resize-y transition-colors duration-150',
      'focus:border-accent placeholder:text-text-dim',
      props.class
    )"
    @input="emit('update:modelValue', ($event.target as HTMLTextAreaElement).value)"
  ></textarea>
</template>
