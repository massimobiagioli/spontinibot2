<script setup lang="ts">
import { ref } from 'vue';

import { DsButton, DsInput } from '../ds';

const props = withDefaults(defineProps<{ busy?: boolean }>(), {
  busy: false,
});

const emit = defineEmits<{ ask: [question: string] }>();

const question = ref('');

function submit(): void {
  const trimmed = question.value.trim();
  if (!trimmed) return;
  emit('ask', trimmed);
  question.value = '';
}
</script>

<template>
  <form class="chat-input" @submit.prevent="submit">
    <DsInput
      v-model="question"
      class="chat-input__field"
      label="La tua domanda"
      placeholder="Scrivi la tua domanda…"
      :disabled="props.busy"
    />
    <DsButton class="chat-input__send" type="submit" :disabled="props.busy">
      {{ props.busy ? 'Invio…' : 'Invia' }}
    </DsButton>
  </form>
</template>
