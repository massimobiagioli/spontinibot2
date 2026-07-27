<script setup lang="ts">
import { computed, ref } from 'vue';

import { DsButton, DsCallout } from '../ds';
import {
  AdminApiError,
  createTrainingFeedback,
  type TrainingFeedbackResponse,
} from '../../services/adminApi';

const props = defineProps<{
  messageId: number;
  answer: string;
  initialFeedback: TrainingFeedbackResponse[];
}>();

const emit = defineEmits<{ changed: [feedback: TrainingFeedbackResponse[]] }>();

const feedback = ref<TrainingFeedbackResponse[]>([...props.initialFeedback]);
const pendingNegative = ref(false);
const reason = ref('');
const submitting = ref(false);
const error = ref<string | null>(null);

const latest = computed(
  () => feedback.value[feedback.value.length - 1] ?? null,
);

function requestNegative(): void {
  error.value = null;
  pendingNegative.value = true;
}

function cancelNegative(): void {
  pendingNegative.value = false;
  reason.value = '';
  error.value = null;
}

async function submit(
  sentiment: 'positive' | 'negative',
  comment?: string,
): Promise<void> {
  submitting.value = true;
  error.value = null;
  try {
    const payload = comment
      ? {
          message_id: props.messageId,
          answer_span: props.answer,
          sentiment,
          comment,
        }
      : {
          message_id: props.messageId,
          answer_span: props.answer,
          sentiment,
        };
    const created = await createTrainingFeedback(payload);
    feedback.value = [...feedback.value, created];
    emit('changed', feedback.value);
    pendingNegative.value = false;
    reason.value = '';
  } catch (e) {
    error.value =
      e instanceof AdminApiError
        ? e.message
        : 'Impossibile registrare il feedback. Riprova più tardi.';
  } finally {
    submitting.value = false;
  }
}

function submitPositive(): void {
  void submit('positive');
}

function confirmNegative(): void {
  const trimmed = reason.value.trim();
  if (!trimmed) {
    error.value = 'Indica il motivo del feedback negativo.';
    return;
  }
  void submit('negative', trimmed);
}
</script>

<template>
  <div class="message-feedback">
    <h3>Feedback</h3>

    <p v-if="latest">
      Ultimo giudizio:
      <span
        class="badge"
        :class="latest.sentiment === 'positive' ? 'badge-success' : 'badge-danger'"
      >
        {{ latest.sentiment === 'positive' ? 'Positivo' : 'Negativo' }}
      </span>
    </p>

    <div class="message-feedback__actions">
      <DsButton
        variant="success"
        :disabled="submitting"
        @click="submitPositive"
      >
        Feedback positivo
      </DsButton>
      <DsButton
        variant="danger"
        outline
        :disabled="submitting"
        @click="requestNegative"
      >
        Feedback negativo
      </DsButton>
    </div>

    <div v-if="pendingNegative" class="message-feedback__reason">
      <label :for="`reason-${messageId}`">Motivazione (obbligatoria)</label>
      <textarea
        :id="`reason-${messageId}`"
        v-model="reason"
        class="form-control"
        rows="2"
        required
      />
      <div class="message-feedback__reason-actions">
        <DsButton
          variant="danger"
          :disabled="submitting"
          @click="confirmNegative"
        >
          Conferma feedback negativo
        </DsButton>
        <DsButton
          variant="secondary"
          outline
          :disabled="submitting"
          @click="cancelNegative"
        >
          Annulla
        </DsButton>
      </div>
    </div>

    <DsCallout v-if="error" variant="danger" title="Errore">
      {{ error }}
    </DsCallout>

    <ul v-if="feedback.length > 0" class="message-feedback__history">
      <li v-for="entry in feedback" :key="entry.id">
        <strong>{{ entry.sentiment === 'positive' ? 'Positivo' : 'Negativo' }}</strong>
        <span v-if="entry.comment"> — {{ entry.comment }}</span>
        <span class="message-feedback__history-date"> ({{ entry.created_at }})</span>
      </li>
    </ul>
  </div>
</template>
