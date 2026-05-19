<script setup lang="ts">
import { inject, type Ref } from 'vue'
import { cn } from '@/lib/utils'

const props = withDefaults(defineProps<{
  class?: string
  align?: 'start' | 'end' | 'center'
}>(), {
  align: 'start',
})

const open = inject<Ref<boolean>>('dropdown-open')

const alignClass: Record<string, string> = {
  start: 'left-0',
  end: 'right-0',
  center: 'left-1/2 -translate-x-1/2',
}
</script>

<template>
  <Transition name="dropdown">
    <div
      v-if="open?.value"
      :class="cn(
        'absolute z-[500] mt-1 min-w-[160px] py-1',
        'bg-surface border border-border rounded-md',
        'shadow-[0_8px_24px_rgba(0,0,0,0.4)]',
        alignClass[props.align],
        props.class
      )"
    >
      <slot />
    </div>
  </Transition>
</template>

<style scoped>
.dropdown-enter-active,
.dropdown-leave-active {
  transition: opacity 0.12s ease, transform 0.12s ease;
}
.dropdown-enter-from,
.dropdown-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}
</style>
