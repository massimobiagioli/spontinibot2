<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';

import { DsButton, DsCallout } from '../ds';
import {
  AdminApiError,
  createTrainingFeedback,
  listTrainingFeedback,
  type TrainingFeedbackResponse,
} from '../../services/adminApi';

const props = defineProps<{
  messageId: number;
  answer: string;
}>();

const segments = computed(() =>
  props.answer.split(/(?<=[.!?])\s+/).filter((segment) => segment.length > 0),
);

const selectedIndex = ref<number | null>(null);
const comment = ref('');
const submitting = ref(false);
const error = ref<string | null>(null);
const feedback = ref<TrainingFeedbackResponse[]>([]);

async function loadFeedback(): Promise<void> {
  try {
    feedback.value = await listTrainingFeedback(props.messageId);
  } catch {
    // The persisted-feedback list is a secondary display; a failed initial
    // fetch simply starts the list empty rather than blocking the answer.
  }
}

onMounted(loadFeedback);

function toggleSegment(index: number): void {
  selectedIndex.value = selectedIndex.value === index ? null : index;
  error.value = null;
  comment.value = '';
}

async function submitFeedback(
  sentiment: 'positive' | 'negative',
): Promise<void> {
  const index = selectedIndex.value;
  if (index === null) return;
  const answerSpan = segments.value[index];
  if (!answerSpan) return;

  submitting.value = true;
  error.value = null;
  try {
    const payload = comment.value
      ? {
          message_id: props.messageId,
          answer_span: answerSpan,
          sentiment,
          comment: comment.value,
        }
      : {
          message_id: props.messageId,
          answer_span: answerSpan,
          sentiment,
        };
    const created = await createTrainingFeedback(payload);
    feedback.value = [...feedback.value, created];
    selectedIndex.value = null;
    comment.value = '';
  } catch (e) {
    error.value =
      e instanceof AdminApiError
        ? e.message
        : 'Impossibile registrare il feedback. Riprova più tardi.';
  } finally {
    submitting.value = false;
  }
}
</script>

<template>
  <div>
    <p>
      <button
        v-for="(segment, index) in segments"
        :key="index"
        type="button"
        class="span-feedback__segment"
        :aria-pressed="selectedIndex === index"
        @click="toggleSegment(index)"
      >
        {{ segment }}
      </button>
    </p>

    <div v-if="selectedIndex !== null">
      <label :for="`comment-${messageId}`">Commento (opzionale)</label>
      <textarea :id="`comment-${messageId}`" v-model="comment" rows="2" />

      <DsButton
        variant="success"
        :disabled="submitting"
        @click="submitFeedback('positive')"
      >
        Feedback positivo
      </DsButton>
      <DsButton
        variant="danger"
        :disabled="submitting"
        @click="submitFeedback('negative')"
      >
        Feedback negativo
      </DsButton>
    </div>

    <DsCallout v-if="error" variant="danger" title="Errore">
      {{ error }}
    </DsCallout>

    <ul v-if="feedback.length > 0">
      <li v-for="entry in feedback" :key="entry.id">
        <strong>{{
          entry.sentiment === 'positive' ? 'Positivo' : 'Negativo'
        }}</strong>
        — "{{ entry.answer_span }}"
        <span v-if="entry.comment"> — {{ entry.comment }}</span>
      </li>
    </ul>
  </div>
</template>
