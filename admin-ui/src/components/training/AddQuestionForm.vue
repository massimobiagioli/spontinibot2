<script setup lang="ts">
import { ref } from 'vue';

import { DsButton, DsCallout, DsInput } from '../ds';
import {
  AdminApiError,
  askTrainingMessage,
  type AskTrainingMessageRequest,
  type TrainingMessageResponse,
} from '../../services/adminApi';

const props = defineProps<{
  sessionId: number;
}>();

const emit = defineEmits<{ added: [message: TrainingMessageResponse] }>();

const question = ref('');
const expectedAnswer = ref('');
const manualMode = ref(false);
const manualAnswer = ref('');
const submitting = ref(false);
const error = ref<string | null>(null);

async function submit(): Promise<void> {
  submitting.value = true;
  error.value = null;
  try {
    const payload: AskTrainingMessageRequest = { question: question.value };
    if (expectedAnswer.value) payload.expected_answer = expectedAnswer.value;
    if (manualMode.value) payload.answer = manualAnswer.value;

    const message = await askTrainingMessage(props.sessionId, payload);
    question.value = '';
    expectedAnswer.value = '';
    manualAnswer.value = '';
    manualMode.value = false;
    emit('added', message);
  } catch (e) {
    error.value =
      e instanceof AdminApiError
        ? e.message
        : 'Impossibile registrare la domanda. Riprova più tardi.';
  } finally {
    submitting.value = false;
  }
}
</script>

<template>
  <form class="add-question-form" @submit.prevent="submit">
    <h2>Aggiungi domanda</h2>

    <DsInput v-model="question" label="Domanda" required />
    <DsInput v-model="expectedAnswer" label="Risposta attesa (opzionale)" />

    <div class="form-check">
      <input
        id="manual-mode"
        v-model="manualMode"
        type="checkbox"
        class="form-check-input"
      />
      <label for="manual-mode" class="form-check-label">
        Inserisci manualmente la risposta del bot, senza interpellarlo dal vivo
      </label>
    </div>

    <div v-if="manualMode" class="form-group">
      <label for="manual-answer">Risposta del bot</label>
      <textarea
        id="manual-answer"
        v-model="manualAnswer"
        class="form-control"
        rows="3"
        required
      />
    </div>

    <DsCallout v-if="error" variant="danger" title="Errore">
      {{ error }}
    </DsCallout>

    <DsButton type="submit" :disabled="submitting">
      {{
        submitting ? 'Invio…' : manualMode ? 'Aggiungi domanda' : 'Chiedi al bot'
      }}
    </DsButton>
  </form>
</template>
