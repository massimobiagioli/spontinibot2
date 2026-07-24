<script setup lang="ts">
import { ref } from 'vue';

import { DsButton, DsCallout } from '../ds';
import { AdminApiError, reloadPersona } from '../../services/adminApi';

const reloading = ref(false);
const error = ref<string | null>(null);
const reloaded = ref(false);

async function reload(): Promise<void> {
  reloading.value = true;
  error.value = null;
  reloaded.value = false;
  try {
    await reloadPersona();
    reloaded.value = true;
  } catch (e) {
    error.value =
      e instanceof AdminApiError
        ? e.message
        : 'Impossibile ricaricare la persona attiva. Riprova più tardi.';
  } finally {
    reloading.value = false;
  }
}
</script>

<template>
  <div>
    <DsButton :disabled="reloading" @click="reload">
      {{ reloading ? 'Ricaricamento…' : 'Ricarica persona attiva' }}
    </DsButton>

    <DsCallout v-if="error" variant="danger" title="Errore">
      {{ error }}
    </DsCallout>

    <DsCallout
      v-else-if="reloaded"
      variant="success"
      title="Persona ricaricata"
    >
      La persona attiva è stata ricaricata dalla cache.
    </DsCallout>
  </div>
</template>
