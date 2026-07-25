<script setup lang="ts">
import { ref } from 'vue';
import { useRouter } from 'vue-router';

import { DsButton, DsCallout, DsInput } from '../components/ds';
import { AdminApiError, login } from '../services/adminApi';

const router = useRouter();

const username = ref('');
const password = ref('');
const submitting = ref(false);
const error = ref<string | null>(null);

async function submit(): Promise<void> {
  submitting.value = true;
  error.value = null;
  try {
    await login(username.value, password.value);
    await router.push({ name: 'ingest' });
  } catch (e) {
    error.value =
      e instanceof AdminApiError
        ? e.message
        : 'Impossibile contattare il server. Riprova più tardi.';
  } finally {
    submitting.value = false;
  }
}
</script>

<template>
  <h1>Accesso operatore</h1>

  <form @submit.prevent="submit">
    <DsInput v-model="username" label="Nome utente" required />

    <DsInput v-model="password" label="Password" type="password" required />

    <DsCallout v-if="error" variant="danger" title="Accesso non riuscito">
      {{ error }}
    </DsCallout>

    <DsButton type="submit" :disabled="submitting">
      {{ submitting ? 'Accesso in corso…' : 'Accedi' }}
    </DsButton>
  </form>
</template>
