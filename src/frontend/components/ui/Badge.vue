<script setup lang="ts">
import { cva, type VariantProps } from 'class-variance-authority'
import { cn } from '@/lib/utils'

const badgeVariants = cva(
  'inline-flex items-center justify-center font-mono border rounded-sm flex-shrink-0',
  {
    variants: {
      variant: {
        default: 'bg-accent-dim border-accent text-accent text-[10px] px-1.5 py-px',
        epic: 'bg-badge-epic-bg border-badge-epic-border text-badge-epic-text text-[9px] font-bold w-[18px] h-[18px]',
        job: 'bg-badge-job-bg border-badge-job-border text-badge-job-text text-[9px] font-bold w-[18px] h-[18px]',
        blocked: 'bg-badge-blocked-bg border-badge-blocked-border text-badge-blocked-text text-[9px] font-bold w-[18px] h-[18px]',
        points: 'bg-accent-dim border-accent text-accent text-[10px] font-semibold px-[7px] py-px rounded-[10px]',
        count: 'bg-surface-2 border-border text-text-dim text-[10px] px-[7px] py-px rounded-[10px]',
        label: 'text-[10px] px-1.5 py-px rounded-sm',
      },
    },
    defaultVariants: {
      variant: 'default',
    },
  }
)

type BadgeVariants = VariantProps<typeof badgeVariants>

const props = withDefaults(defineProps<{
  variant?: NonNullable<BadgeVariants['variant']>
  class?: string
}>(), {
  variant: 'default',
})
</script>

<template>
  <span :class="cn(badgeVariants({ variant: props.variant }), props.class)">
    <slot />
  </span>
</template>
