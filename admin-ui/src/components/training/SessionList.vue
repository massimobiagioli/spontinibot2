<script setup lang="ts">
import { ref } from 'vue';

import { DsButton, DsConfirmDialog, DsInput } from '../ds';
import {
  AdminApiError,
  closeSession,
  createSession,
  type TrainingSessionResponse,
} from '../../services/adminApi';

defineProps<{
  sessions: TrainingSessionResponse[];
}>();

const emit = defineEmits<{ changed: [] }>();

const newSessionTitle = ref('');
const addError = ref<string | null>(null);
const adding = ref(false);

async function addSession(): Promise<void> {
  adding.value = true;
  addError.value = null;
  try {
    await createSession(newSessionTitle.value);
    newSessionTitle.value = '';
    emit('changed');
  } catch (e) {
    addError.value =
      e instanceof AdminApiError
        ? e.message
        : 'Impossibile creare la sessione. Riprova più tardi.';
  } finally {
    adding.value = false;
  }
}

const pendingCloseId = ref<number | null>(null);
const closeError = ref<string | null>(null);

function requestClose(id: number): void {
  closeError.value = null;
  pendingCloseId.value = id;
}

function cancelClose(): void {
  pendingCloseId.value = null;
}

async function confirmClose(): Promise<void> {
  const id = pendingCloseId.value;
  if (id === null) return;
  try {
    await closeSession(id);
    pendingCloseId.value = null;
    emit('changed');
  } catch (e) {
    closeError.value =
      e instanceof AdminApiError
        ? e.message
        : 'Impossibile chiudere la sessione. Riprova più tardi.';
    pendingCloseId.value = null;
  }
}
</script>

<template>
  <section>
    <h2>Sessioni</h2>

    <p v-if="closeError" role="alert">{{ closeError }}</p>

    <ul>
      <li v-for="session in sessions" :key="session.id">
        <RouterLink :to="`/training/${session.id}`">
          {{ session.title }}
        </RouterLink>
        <span> — {{ session.created_at }}</span>
        <span v-if="session.closed_at" class="badge badge-secondary">
          Chiusa
        </span>
        <DsButton
          v-else
          variant="danger"
          outline
          @click="requestClose(session.id)"
        >
          Chiudi sessione
        </DsButton>
      </li>
    </ul>

    <form @submit.prevent="addSession">
      <h3>Nuova sessione</h3>
      <DsInput v-model="newSessionTitle" label="Titolo sessione" required />
      <p v-if="addError" role="alert">{{ addError }}</p>
      <DsButton type="submit" :disabled="adding">
        {{ adding ? 'Creazione…' : 'Crea sessione' }}
      </DsButton>
    </form>

    <DsConfirmDialog
      data-testid="close-session-dialog"
      :open="pendingCloseId !== null"
      message="Chiudere questa sessione? Non potrà più essere riaperta."
      confirm-label="Chiudi"
      @confirm="confirmClose"
      @cancel="cancelClose"
    />
  </section>
</template>
