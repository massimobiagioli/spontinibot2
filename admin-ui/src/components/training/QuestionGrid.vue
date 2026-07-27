<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';

import QuestionDetail from './QuestionDetail.vue';
import {
  listTrainingFeedback,
  type TrainingFeedbackResponse,
  type TrainingMessageResponse,
} from '../../services/adminApi';

const props = defineProps<{
  messages: TrainingMessageResponse[];
}>();

const feedbackByMessage = ref<Record<number, TrainingFeedbackResponse[]>>({});
const selectedId = ref<number | null>(null);

async function loadFeedback(): Promise<void> {
  const entries = await Promise.all(
    props.messages.map(async (message) => {
      try {
        return [message.id, await listTrainingFeedback(message.id)] as const;
      } catch {
        // The feedback badge is a secondary display; a failed fetch simply
        // shows "no feedback" for that card rather than blocking the grid.
        return [message.id, []] as const;
      }
    }),
  );
  feedbackByMessage.value = Object.fromEntries(entries);
}

onMounted(loadFeedback);
watch(
  () => props.messages.map((m) => m.id).join(','),
  loadFeedback,
);

type Verdict = 'positive' | 'negative' | 'none';

function verdictFor(messageId: number): Verdict {
  const list = feedbackByMessage.value[messageId];
  if (!list || list.length === 0) return 'none';
  return list[list.length - 1]!.sentiment === 'positive' ? 'positive' : 'negative';
}

function verdictLabel(verdict: Verdict): string {
  if (verdict === 'positive') return 'Positivo';
  if (verdict === 'negative') return 'Negativo';
  return 'Nessun feedback';
}

const selectedMessage = computed(
  () => props.messages.find((m) => m.id === selectedId.value) ?? null,
);

function open(id: number): void {
  selectedId.value = id;
}

function close(): void {
  selectedId.value = null;
}

function onFeedbackChanged(
  messageId: number,
  list: TrainingFeedbackResponse[],
): void {
  feedbackByMessage.value = { ...feedbackByMessage.value, [messageId]: list };
}
</script>

<template>
  <section>
    <h2>Domande ({{ messages.length }})</h2>

    <p v-if="messages.length === 0">
      Nessuna domanda in questa sessione.
    </p>

    <div v-else class="question-grid">
      <button
        v-for="message in messages"
        :key="message.id"
        type="button"
        class="question-grid__card"
        @click="open(message.id)"
      >
        <p class="question-grid__question">{{ message.question }}</p>
        <div class="question-grid__meta">
          <span class="badge badge-secondary">
            {{ message.source === 'manual' ? 'Manuale' : 'Bot' }}
          </span>
          <span v-if="message.fell_back" class="badge badge-warning">
            Nessuna info trovata
          </span>
          <span v-if="message.execution_time_ms !== null" class="question-grid__time">
            {{ message.execution_time_ms }} ms
          </span>
        </div>
        <span
          class="badge"
          :class="{
            'badge-success': verdictFor(message.id) === 'positive',
            'badge-danger': verdictFor(message.id) === 'negative',
            'badge-light': verdictFor(message.id) === 'none',
          }"
        >
          {{ verdictLabel(verdictFor(message.id)) }}
        </span>
      </button>
    </div>

    <QuestionDetail
      v-if="selectedMessage"
      :message="selectedMessage"
      :feedback="feedbackByMessage[selectedMessage.id] ?? []"
      @close="close"
      @feedback-changed="onFeedbackChanged"
    />
  </section>
</template>

<style scoped>
.question-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
  gap: 1rem;
}

.question-grid__card {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.5rem;
  text-align: left;
  padding: 1rem;
  border: 1px solid rgba(31, 42, 55, 0.12);
  border-radius: 8px;
  background: var(--spontini-color-white, #fff);
  cursor: pointer;
}

.question-grid__card:hover,
.question-grid__card:focus-visible {
  border-color: var(--spontini-color-primary);
}

.question-grid__question {
  font-weight: 600;
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
  margin: 0;
}

.question-grid__meta {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  align-items: center;
}

.question-grid__time {
  font-size: 0.85rem;
  color: var(--spontini-color-text-muted, #6c757d);
}
</style>
