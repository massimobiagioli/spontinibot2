<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useRoute } from 'vue-router';

import { DsCallout } from '../components/ds';
import AddQuestionForm from '../components/training/AddQuestionForm.vue';
import CloseSessionForm from '../components/training/CloseSessionForm.vue';
import QuestionGrid from '../components/training/QuestionGrid.vue';
import {
  AdminApiError,
  getSession,
  listTrainingMessages,
  type TrainingMessageResponse,
  type TrainingSessionResponse,
} from '../services/adminApi';

const route = useRoute();
const sessionId = computed(() => Number(route.params['id']));

const session = ref<TrainingSessionResponse | null>(null);
const messages = ref<TrainingMessageResponse[]>([]);
const loading = ref(true);
const error = ref<string | null>(null);

async function loadSession(): Promise<void> {
  loading.value = true;
  error.value = null;
  try {
    const [sessionResult, messagesResult] = await Promise.all([
      getSession(sessionId.value),
      listTrainingMessages(sessionId.value),
    ]);
    session.value = sessionResult;
    messages.value = messagesResult;
  } catch (e) {
    error.value =
      e instanceof AdminApiError
        ? e.message
        : 'Impossibile contattare il server. Riprova più tardi.';
  } finally {
    loading.value = false;
  }
}

onMounted(loadSession);

function onAdded(message: TrainingMessageResponse): void {
  messages.value = [...messages.value, message];
}

function onClosed(): void {
  void loadSession();
}
</script>

<template>
  <p v-if="loading">Caricamento della sessione…</p>

  <DsCallout
    v-else-if="error"
    variant="danger"
    title="Impossibile caricare la sessione"
  >
    {{ error }}
  </DsCallout>

  <template v-else-if="session">
    <h1>{{ session.title }}</h1>

    <DsCallout v-if="session.closed_at" variant="primary" title="Sessione chiusa">
      <p>Chiusa il {{ session.closed_at }}</p>
      <p v-if="session.notes">Note: {{ session.notes }}</p>
    </DsCallout>

    <template v-if="!session.closed_at">
      <AddQuestionForm :session-id="sessionId" @added="onAdded" />
      <CloseSessionForm :session-id="sessionId" @closed="onClosed" />
    </template>

    <QuestionGrid :messages="messages" />
  </template>
</template>
