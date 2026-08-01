<script setup lang="ts">
import { computed } from 'vue';

const props = withDefaults(
  defineProps<{
    title: string;
    defaultOpen?: boolean;
    /**
     * The accordion title doubles as a document heading (matching Bootstrap
     * Italia's own accordion markup, which wraps its trigger in a heading
     * element) so the page keeps a correct, unbroken heading order — set
     * this to whatever level is next after the heading this accordion sits
     * under (e.g. 3 when nesting under a dialog's own <h2>).
     */
    headingLevel?: 2 | 3 | 4;
  }>(),
  {
    defaultOpen: false,
    headingLevel: 2,
  },
);

const headingTag = computed(() => `h${props.headingLevel}`);
</script>

<template>
  <details class="ds-accordion" :open="defaultOpen">
    <summary class="ds-accordion__summary">
      <component :is="headingTag" class="ds-accordion__title">{{
        title
      }}</component>
    </summary>
    <div class="ds-accordion__body">
      <slot />
    </div>
  </details>
</template>

<style scoped>
.ds-accordion {
  border: 1px solid rgba(31, 42, 55, 0.12);
  border-radius: 8px;
  padding: 0.75rem 1rem;
}

.ds-accordion + .ds-accordion {
  margin-top: 1rem;
}

.ds-accordion__summary {
  cursor: pointer;
  list-style: none;
}

.ds-accordion__summary::-webkit-details-marker {
  display: none;
}

.ds-accordion__title {
  display: inline;
  margin: 0;
  font-size: 1.1rem;
  font-weight: 600;
}

.ds-accordion__summary::before {
  content: '\25B8';
  display: inline-block;
  margin-right: 0.5rem;
  transition: transform 0.15s ease;
}

.ds-accordion[open] > .ds-accordion__summary::before {
  transform: rotate(90deg);
}

.ds-accordion__body {
  margin-top: 0.75rem;
}
</style>
