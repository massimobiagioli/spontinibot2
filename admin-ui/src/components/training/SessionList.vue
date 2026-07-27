<script setup lang="ts">
import { ref } from 'vue';

import { DsButton, DsConfirmDialog, DsInput } from '../ds';
import {
  AdminApiError,
  createSession,
  deleteSession,
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

const pendingDeleteId = ref<number | null>(null);
const deleteError = ref<string | null>(null);

function requestDelete(id: number): void {
  deleteError.value = null;
  pendingDeleteId.value = id;
}

function cancelDelete(): void {
  pendingDeleteId.value = null;
}

async function confirmDelete(): Promise<void> {
  const id = pendingDeleteId.value;
  if (id === null) return;
  try {
    await deleteSession(id);
    pendingDeleteId.value = null;
    emit('changed');
  } catch (e) {
    deleteError.value =
      e instanceof AdminApiError
        ? e.message
        : 'Impossibile eliminare la sessione. Riprova più tardi.';
    pendingDeleteId.value = null;
  }
}
</script>

<template>
  <section>
    <h2>Sessioni</h2>

    <p v-if="deleteError" role="alert">{{ deleteError }}</p>

    <ul>
      <li v-for="session in sessions" :key="session.id">
        <RouterLink :to="`/training/${session.id}`">
          {{ session.title }}
        </RouterLink>
        <span> — {{ session.created_at }}</span>
        <span v-if="session.closed_at" class="badge badge-secondary">
          Chiusa
        </span>
        <span v-else class="badge badge-success">Aperta</span>
        <DsButton
          variant="danger"
          outline
          @click="requestDelete(session.id)"
        >
          Elimina sessione
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
      data-testid="delete-session-dialog"
      :open="pendingDeleteId !== null"
      message="Eliminare questa sessione e tutte le sue domande? L'operazione non può essere annullata."
      confirm-label="Elimina"
      @confirm="confirmDelete"
      @cancel="cancelDelete"
    />
  </section>
</template>
