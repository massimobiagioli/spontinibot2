<script setup lang="ts">
import { onMounted, ref } from 'vue';

import { DsButton, DsCallout } from '../ds';
import MessageFeedback from './MessageFeedback.vue';
import type {
  TrainingFeedbackResponse,
  TrainingMessageResponse,
} from '../../services/adminApi';

const props = defineProps<{
  message: TrainingMessageResponse;
  feedback: TrainingFeedbackResponse[];
}>();

const emit = defineEmits<{
  close: [];
  'feedback-changed': [messageId: number, feedback: TrainingFeedbackResponse[]];
}>();

const dialogRef = ref<HTMLDialogElement | null>(null);

function isLink(sourceRef: string): boolean {
  return /^https?:\/\//.test(sourceRef);
}

onMounted(() => {
  if (typeof dialogRef.value?.showModal === 'function') {
    dialogRef.value.showModal();
  }
});

function close(): void {
  emit('close');
}

function onFeedbackChanged(list: TrainingFeedbackResponse[]): void {
  emit('feedback-changed', props.message.id, list);
}
</script>

<template>
  <dialog
    ref="dialogRef"
    class="question-detail"
    @cancel.prevent="close"
  >
    <div class="question-detail__content">
      <header class="question-detail__header">
        <h2>Scheda domanda</h2>
        <button
          type="button"
          class="question-detail__close-icon"
          aria-label="Chiudi"
          @click="close"
        >
          &times;
        </button>
      </header>

      <div class="question-detail__body">
        <section class="question-detail__field">
          <p class="question-detail__label">Domanda</p>
          <p class="question-detail__value">{{ message.question }}</p>
        </section>

        <section
          v-if="message.expected_answer"
          class="question-detail__field question-detail__field--expected"
        >
          <p class="question-detail__label">Risposta attesa</p>
          <p class="question-detail__value">{{ message.expected_answer }}</p>
        </section>

        <section class="question-detail__field">
          <p class="question-detail__label">Risposta del bot</p>
          <p class="question-detail__value">{{ message.answer }}</p>
        </section>

        <dl class="question-detail__meta">
          <div class="question-detail__meta-item">
            <dt>Tempo di esecuzione</dt>
            <dd>
              {{
                message.execution_time_ms !== null
                  ? `${message.execution_time_ms} ms`
                  : 'n/d (inserita manualmente)'
              }}
            </dd>
          </div>
          <div class="question-detail__meta-item">
            <dt>Origine</dt>
            <dd>{{ message.source === 'manual' ? 'Manuale' : 'Domanda al bot' }}</dd>
          </div>
        </dl>

        <DsCallout
          v-if="message.fell_back"
          variant="warning"
          title="Nessuna informazione trovata"
        >
          Spontini non ha trovato informazioni nei documenti comunali per
          rispondere a questa domanda.
        </DsCallout>
        <details v-else class="question-detail__accordion">
          <summary>Fonti ({{ message.sources.length }})</summary>
          <ul class="question-detail__sources">
            <li v-for="source in message.sources" :key="source.document_id">
              <a
                v-if="isLink(source.source_ref)"
                :href="source.source_ref"
                target="_blank"
                rel="noopener noreferrer"
              >
                {{ source.source_ref }}
              </a>
              <span v-else>{{ source.source_ref }}</span>
            </li>
          </ul>
        </details>

        <div class="question-detail__feedback">
          <MessageFeedback
            :message-id="message.id"
            :answer="message.answer"
            :initial-feedback="feedback"
            @changed="onFeedbackChanged"
          />
        </div>
      </div>

      <footer class="question-detail__footer">
        <DsButton variant="secondary" outline @click="close">Chiudi</DsButton>
      </footer>
    </div>
  </dialog>
</template>

<style scoped>
.question-detail {
  border: none;
  border-radius: 8px;
  padding: 0;
  width: min(40rem, 90vw);
  max-height: 85vh;
  overflow: hidden;
}

.question-detail::backdrop {
  background: rgba(0, 0, 0, 0.5);
}

.question-detail__content {
  display: flex;
  flex-direction: column;
  max-height: 85vh;
}

.question-detail__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 1.25rem 1.5rem;
  border-bottom: 1px solid rgba(31, 42, 55, 0.08);
}

.question-detail__header h2 {
  margin: 0;
  font-size: 1.25rem;
}

.question-detail__close-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 44px;
  min-width: 44px;
  border: none;
  border-radius: 4px;
  background: none;
  font-size: 1.5rem;
  line-height: 1;
  color: var(--spontini-color-text-muted, #6c757d);
  cursor: pointer;
}

.question-detail__close-icon:hover,
.question-detail__close-icon:focus-visible {
  background: rgba(31, 42, 55, 0.06);
  color: var(--spontini-color-text, #1f2a37);
}

.question-detail__body {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
  padding: 1.5rem;
  overflow-y: auto;
}

.question-detail__field {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.question-detail__field--expected {
  background: var(--spontini-color-bg, #f4f6f9);
  border-radius: 8px;
  padding: 0.75rem 1rem;
}

.question-detail__label {
  margin: 0;
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--spontini-color-text-muted, #6c757d);
}

.question-detail__value {
  margin: 0;
  line-height: 1.5;
}

.question-detail__meta {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem 1.5rem;
  margin: 0;
  font-size: 0.85rem;
  color: var(--spontini-color-text-muted, #6c757d);
}

.question-detail__meta-item {
  display: flex;
  gap: 0.35rem;
}

.question-detail__meta-item dt {
  font-weight: 600;
}

.question-detail__meta-item dd {
  margin: 0;
}

.question-detail__accordion {
  border: 1px solid rgba(31, 42, 55, 0.12);
  border-radius: 8px;
  padding: 0.75rem 1rem;
}

.question-detail__accordion summary {
  cursor: pointer;
  font-weight: 600;
  list-style: none;
}

.question-detail__accordion summary::-webkit-details-marker {
  display: none;
}

.question-detail__accordion summary::before {
  content: '\25B8';
  display: inline-block;
  margin-right: 0.5rem;
  transition: transform 0.15s ease;
}

.question-detail__accordion[open] summary::before {
  transform: rotate(90deg);
}

.question-detail__sources {
  margin: 0.75rem 0 0;
  padding-left: 1.25rem;
}

.question-detail__feedback {
  border-top: 1px solid rgba(31, 42, 55, 0.08);
  padding-top: 1.25rem;
}

.question-detail__footer {
  display: flex;
  justify-content: flex-end;
  padding: 1rem 1.5rem;
  border-top: 1px solid rgba(31, 42, 55, 0.08);
}
</style>
