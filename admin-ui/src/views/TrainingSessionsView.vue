<script setup lang="ts">
import { onMounted, ref } from 'vue';

import { DsCallout } from '../components/ds';
import SessionList from '../components/training/SessionList.vue';
import {
  AdminApiError,
  listSessions,
  type TrainingSessionResponse,
} from '../services/adminApi';

const sessions = ref<TrainingSessionResponse[] | null>(null);
const loading = ref(true);
const error = ref<string | null>(null);

async function loadSessions(): Promise<void> {
  loading.value = true;
  error.value = null;
  try {
    sessions.value = await listSessions();
  } catch (e) {
    error.value =
      e instanceof AdminApiError
        ? e.message
        : 'Impossibile contattare il server. Riprova più tardi.';
  } finally {
    loading.value = false;
  }
}

onMounted(loadSessions);

function onChanged(): void {
  void loadSessions();
}
</script>

<template>
  <h1>Training</h1>

  <p v-if="loading">Caricamento delle sessioni…</p>

  <DsCallout
    v-else-if="error"
    variant="danger"
    title="Impossibile caricare le sessioni"
  >
    {{ error }}
  </DsCallout>

  <template v-else-if="sessions">
    <DsCallout
      v-if="sessions.length === 0"
      variant="primary"
      title="Nessuna sessione"
    >
      Non esiste ancora nessuna sessione di training. Creane una qui sotto.
    </DsCallout>
    <p v-else>{{ sessions.length }} sessioni totali</p>

    <SessionList :sessions="sessions" @changed="onChanged" />
  </template>
</template>
