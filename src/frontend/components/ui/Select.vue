<script setup lang="ts">
import { cn } from '@/lib/utils'
import { ref, computed } from 'vue'

const props = withDefaults(defineProps<{
  modelValue?: string
  placeholder?: string
  disabled?: boolean
  class?: string
  options?: { value: string; label: string }[]
}>(), {
  placeholder: 'Auswählen…',
  options: () => [],
})

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
}>()

const open = ref(false)

const selectedLabel = computed(() => {
  const opt = props.options.find(o => o.value === props.modelValue)
  return opt ? opt.label : props.placeholder
})

function select(value: string) {
  emit('update:modelValue', value)
  open.value = false
}

function onBlur() {
  setTimeout(() => { open.value = false }, 150)
}
</script>

<template>
  <div class="relative w-full" @blur.capture="onBlur">
    <button
      type="button"
      :disabled="props.disabled"
      :class="cn(
        'w-full flex items-center justify-between gap-2',
        'rounded-md border border-border bg-surface-2 px-3 py-2',
        'text-[13px] text-text font-sans',
        'hover:border-accent/60 focus:outline-none focus:border-accent',
        'disabled:opacity-50 disabled:cursor-not-allowed',
        'transition-colors duration-150',
        props.class
      )"
      @click="open = !open"
    >
      <span :class="!props.modelValue ? 'text-text-dim' : ''">
        {{ selectedLabel }}
      </span>
      <svg
        :class="['w-3.5 h-3.5 text-text-dim transition-transform duration-150 flex-shrink-0', open && 'rotate-180']"
        viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
      >
        <polyline points="6 9 12 15 18 9" />
      </svg>
    </button>

    <Transition name="select-dropdown">
      <div
        v-if="open"
        class="absolute z-[200] left-0 right-0 mt-1 py-1 bg-surface border border-border rounded-md shadow-[0_4px_16px_rgba(0,0,0,0.4)] max-h-52 overflow-y-auto"
      >
        <slot>
          <button
            v-for="opt in props.options"
            :key="opt.value"
            type="button"
            :class="cn(
              'w-full text-left px-3 py-2 text-[13px] font-sans',
              'hover:bg-surface-2 transition-colors duration-100',
              props.modelValue === opt.value ? 'text-accent' : 'text-text'
            )"
            @click="select(opt.value)"
          >
            {{ opt.label }}
          </button>
        </slot>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.select-dropdown-enter-active,
.select-dropdown-leave-active {
  transition: opacity 0.12s ease, transform 0.12s ease;
}
.select-dropdown-enter-from,
.select-dropdown-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}
</style>
