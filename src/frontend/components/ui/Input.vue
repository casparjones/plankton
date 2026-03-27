<script setup lang="ts">
import { cn } from '@/lib/utils'

const props = withDefaults(defineProps<{
  class?: string
  modelValue?: string | number
  type?: string
}>(), {
  type: 'text',
})

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
}>()
</script>

<template>
  <input
    :type="props.type"
    :value="props.modelValue"
    :class="cn(
      'w-full bg-surface-2 border border-border rounded-md text-text text-sm',
      'px-3 py-2 font-sans outline-none transition-colors duration-150',
      'focus:border-accent placeholder:text-text-dim',
      props.class
    )"
    @input="emit('update:modelValue', ($event.target as HTMLInputElement).value)"
  />
</template>
