<script setup lang="ts">
import { ref } from 'vue';

import { DsButton, DsCallout, DsInput } from '../ds';
import {
  AdminApiError,
  askTrainingMessage,
  type TrainingMessageResponse,
} from '../../services/adminApi';

const props = defineProps<{
  sessionId: number;
}>();

const emit = defineEmits<{ asked: [message: TrainingMessageResponse] }>();

const question = ref('');
const asking = ref(false);
const error = ref<string | null>(null);

async function ask(): Promise<void> {
  asking.value = true;
  error.value = null;
  try {
    const message = await askTrainingMessage(props.sessionId, question.value);
    question.value = '';
    emit('asked', message);
  } catch (e) {
    error.value =
      e instanceof AdminApiError
        ? e.message
        : 'Impossibile ottenere una risposta. Riprova più tardi.';
  } finally {
    asking.value = false;
  }
}
</script>

<template>
  <form @submit.prevent="ask">
    <DsInput v-model="question" label="Domanda" required />

    <DsCallout v-if="error" variant="danger" title="Errore">
      {{ error }}
    </DsCallout>

    <DsButton type="submit" :disabled="asking">
      {{ asking ? 'Invio…' : 'Chiedi' }}
    </DsButton>
  </form>
</template>
