<script setup lang="ts">
type Variant = 'primary' | 'success' | 'warning' | 'danger';

withDefaults(
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
</script>

<template>
  <div
    class="callout"
    :class="[`callout-${variant}`, { 'callout-highlight': highlight }]"
    role="note"
  >
    <div class="callout-inner">
      <p v-if="title" class="callout-title">{{ title }}</p>
      <slot />
    </div>
  </div>
</template>
