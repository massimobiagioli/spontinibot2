<script setup lang="ts">
import { computed, onUnmounted, ref } from 'vue';

import { DsButton, DsCallout } from '../ds';
import {
  AdminApiError,
  getIngestRun,
  triggerIngestRun,
} from '../../services/adminApi';

const POLL_INTERVAL_MS = 2000;
const TERMINAL_STATUSES = new Set(['done', 'failed']);

const runId = ref<number | null>(null);
const status = ref<string | null>(null);
const error = ref<string | null>(null);
const triggering = ref(false);
let intervalId: ReturnType<typeof setInterval> | undefined;

function stopPolling(): void {
  if (intervalId !== undefined) {
    clearInterval(intervalId);
    intervalId = undefined;
  }
}

async function poll(): Promise<void> {
  if (runId.value === null) return;
  try {
    const run = await getIngestRun(runId.value);
    status.value = run.status;
    if (TERMINAL_STATUSES.has(run.status)) stopPolling();
  } catch (e) {
    error.value =
      e instanceof AdminApiError
        ? e.message
        : "Impossibile verificare lo stato dell'esecuzione.";
    stopPolling();
  }
}

function startPolling(): void {
  stopPolling();
  intervalId = setInterval(poll, POLL_INTERVAL_MS);
}

async function trigger(): Promise<void> {
  triggering.value = true;
  error.value = null;
  try {
    const run = await triggerIngestRun();
    runId.value = run.id;
    status.value = run.status;
    startPolling();
  } catch (e) {
    error.value =
      e instanceof AdminApiError
        ? e.message
        : "Impossibile avviare l'esecuzione. Riprova più tardi.";
  } finally {
    triggering.value = false;
  }
}

const calloutVariant = computed(() => {
  if (status.value === 'done') return 'success';
  if (status.value === 'failed') return 'danger';
  return 'warning';
});

onUnmounted(stopPolling);
</script>

<template>
  <div>
    <DsButton :disabled="triggering" @click="trigger">
      {{ triggering ? 'Avvio…' : 'Esegui ora' }}
    </DsButton>

    <DsCallout v-if="error" variant="danger" title="Errore">
      {{ error }}
    </DsCallout>

    <DsCallout
      v-else-if="status"
      :variant="calloutVariant"
      title="Stato esecuzione"
    >
      {{ status }}
    </DsCallout>
  </div>
</template>
