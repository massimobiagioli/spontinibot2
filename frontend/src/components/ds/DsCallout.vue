<script setup lang="ts">
import { computed } from 'vue';

type Variant = 'primary' | 'success' | 'warning' | 'danger';

const props = withDefaults(
  defineProps<{
    variant?: Variant;
    title?: string;
    highlight?: boolean;
  }>(),
  {
    variant: 'primary',
    highlight: false,
  },
);

// Errors and honest-unknown/fallback messages appear dynamically, without
// moving focus, so they need a live-region role to be announced (WCAG 4.1.3
// Status Messages) — `note` is a static role and is never auto-announced.
const role = computed(() => {
  if (props.variant === 'danger') return 'alert';
  if (props.variant === 'success') return 'status';
  return 'note';
});
</script>

<template>
  <div
    class="callout"
    :class="[`callout-${variant}`, { 'callout-highlight': highlight }]"
    :role="role"
  >
    <div class="callout-inner">
      <p v-if="title" class="callout-title">{{ title }}</p>
      <slot />
    </div>
  </div>
</template>
