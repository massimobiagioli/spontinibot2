<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useRoute } from 'vue-router';

import { DsCallout } from '../components/ds';
import AskAnswerBox from '../components/training/AskAnswerBox.vue';
import MessageList from '../components/training/MessageList.vue';
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

function onAsked(message: TrainingMessageResponse): void {
  messages.value = [message, ...messages.value];
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
    <p v-if="session.closed_at">Sessione chiusa il {{ session.closed_at }}</p>

    <AskAnswerBox
      v-if="!session.closed_at"
      :session-id="sessionId"
      @asked="onAsked"
    />

    <MessageList :messages="messages" />
  </template>
</template>
