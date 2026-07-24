<script setup lang="ts">
import { computed } from 'vue';

type Variant =
  'primary' | 'secondary' | 'success' | 'danger' | 'warning' | 'light' | 'link';
type Size = 'sm' | 'lg';

const props = withDefaults(
  defineProps<{
    variant?: Variant;
    outline?: boolean;
    size?: Size;
    disabled?: boolean;
    type?: 'button' | 'submit' | 'reset';
  }>(),
  {
    variant: 'primary',
    outline: false,
    disabled: false,
    type: 'button',
  },
);

defineEmits<{ click: [event: MouseEvent] }>();

const classes = computed(() => [
  props.outline ? `btn-outline-${props.variant}` : `btn-${props.variant}`,
  props.size ? `btn-${props.size}` : null,
]);
</script>

<template>
  <button
    :type="type"
    class="btn"
    :class="classes"
    :disabled="disabled"
    @click="$emit('click', $event)"
  >
    <slot />
  </button>
</template>
