<script setup lang="ts">
import { useId } from 'vue';

withDefaults(
  defineProps<{
    modelValue: string;
    label: string;
    type?: string;
    hint?: string;
    placeholder?: string;
    disabled?: boolean;
    required?: boolean;
  }>(),
  {
    type: 'text',
    disabled: false,
    required: false,
  },
);

defineEmits<{ 'update:modelValue': [value: string] }>();

const inputId = useId();
const hintId = useId();
</script>

<template>
  <div class="form-group">
    <label :for="inputId">{{ label }}</label>
    <input
      :id="inputId"
      class="form-control"
      :type="type"
      :value="modelValue"
      :placeholder="placeholder"
      :disabled="disabled"
      :required="required"
      :aria-describedby="hint ? hintId : undefined"
      @input="
        $emit('update:modelValue', ($event.target as HTMLInputElement).value)
      "
    />
    <small v-if="hint" :id="hintId" class="form-text">{{ hint }}</small>
  </div>
</template>
