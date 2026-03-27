<script setup lang="ts">
import { cva, type VariantProps } from 'class-variance-authority'
import { cn } from '@/lib/utils'

const buttonVariants = cva(
  'inline-flex items-center justify-center rounded-md font-sans cursor-pointer transition-all duration-150 disabled:opacity-50 disabled:cursor-not-allowed',
  {
    variants: {
      variant: {
        default: 'bg-accent border-none text-white font-semibold hover:opacity-85',
        danger: 'bg-transparent border border-danger text-danger hover:bg-danger/10',
        outline: 'bg-transparent border border-border text-text-dim hover:border-accent hover:text-accent',
        ghost: 'bg-transparent text-text-dim hover:bg-surface-2',
        mcp: 'bg-surface-2 border border-border text-text-dim font-mono hover:border-accent hover:text-accent',
      },
      size: {
        default: 'px-5 py-2 text-[13px]',
        sm: 'px-3.5 py-1.5 text-xs',
        xs: 'px-2 py-0.5 text-xs',
        icon: 'h-[22px] px-1.5 text-base',
      },
    },
    defaultVariants: {
      variant: 'default',
      size: 'default',
    },
  }
)

type ButtonVariants = VariantProps<typeof buttonVariants>

const props = withDefaults(defineProps<{
  variant?: NonNullable<ButtonVariants['variant']>
  size?: NonNullable<ButtonVariants['size']>
  class?: string
  disabled?: boolean
  type?: 'button' | 'submit' | 'reset'
}>(), {
  variant: 'default',
  size: 'default',
  type: 'button',
})
</script>

<template>
  <button
    :type="props.type"
    :disabled="props.disabled"
    :class="cn(buttonVariants({ variant: props.variant, size: props.size }), props.class)"
  >
    <slot />
  </button>
</template>
