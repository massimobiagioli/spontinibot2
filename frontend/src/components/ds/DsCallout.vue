<script setup lang="ts">
import { computed } from 'vue';

type Variant = 'primary' | 'success' | 'warning' | 'danger';
type CalloutRole = 'alert' | 'status' | 'note';

const props = withDefaults(
  defineProps<{
    variant?: Variant;
    title?: string;
    highlight?: boolean;
    role?: CalloutRole;
  }>(),
  {
    variant: 'primary',
    highlight: false,
  },
);

// Errors and honest-unknown/fallback messages appear dynamically, without
// moving focus, so they need a live-region role to be announced (WCAG 4.1.3
// Status Messages) — `note` is a static role and is never auto-announced.
// `role` lets a consumer request `status`/`alert` for a variant whose default
// mapping is `note` (e.g. a "primary"-styled honest-unknown message that must
// still be announced) without changing that variant's default for other,
// purely decorative consumers.
const computedRole = computed<CalloutRole>(() => {
  if (props.role) return props.role;
  if (props.variant === 'danger') return 'alert';
  if (props.variant === 'success') return 'status';
  return 'note';
});
</script>

<template>
  <div
    class="callout"
    :class="[`callout-${variant}`, { 'callout-highlight': highlight }]"
    :role="computedRole"
  >
    <div class="callout-inner">
      <p v-if="title" class="callout-title">{{ title }}</p>
      <slot />
    </div>
  </div>
</template>
