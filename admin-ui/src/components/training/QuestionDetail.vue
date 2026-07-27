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
    class="modal question-detail"
    open
    @cancel.prevent="close"
  >
    <div class="modal-content">
      <div class="modal-header">
        <h2>Scheda domanda</h2>
      </div>
      <div class="modal-body question-detail__body">
        <p><strong>Domanda:</strong> {{ message.question }}</p>
        <p v-if="message.expected_answer">
          <strong>Risposta attesa:</strong> {{ message.expected_answer }}
        </p>
        <p><strong>Risposta del bot:</strong> {{ message.answer }}</p>
        <p>
          <strong>Tempo di esecuzione:</strong>
          {{
            message.execution_time_ms !== null
              ? `${message.execution_time_ms} ms`
              : 'n/d (inserita manualmente)'
          }}
        </p>
        <p>
          <strong>Origine:</strong>
          {{ message.source === 'manual' ? 'Manuale' : 'Domanda al bot' }}
        </p>

        <DsCallout
          v-if="message.fell_back"
          variant="warning"
          title="Nessuna informazione trovata"
        >
          Spontini non ha trovato informazioni nei documenti comunali per
          rispondere a questa domanda.
        </DsCallout>
        <details v-else>
          <summary>Fonti ({{ message.sources.length }})</summary>
          <ul>
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

        <MessageFeedback
          :message-id="message.id"
          :answer="message.answer"
          :initial-feedback="feedback"
          @changed="onFeedbackChanged"
        />
      </div>
      <div class="modal-footer">
        <DsButton variant="secondary" outline @click="close">Chiudi</DsButton>
      </div>
    </div>
  </dialog>
</template>
