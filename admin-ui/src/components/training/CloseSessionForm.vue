<script setup lang="ts">
import { ref } from 'vue';

import { DsButton, DsCallout, DsConfirmDialog } from '../ds';
import { AdminApiError, closeSession } from '../../services/adminApi';

const props = defineProps<{
  sessionId: number;
}>();

const emit = defineEmits<{ closed: [] }>();

const notes = ref('');
const requesting = ref(false);
const closing = ref(false);
const error = ref<string | null>(null);

function requestClose(): void {
  error.value = null;
  requesting.value = true;
}

function cancel(): void {
  requesting.value = false;
}

async function confirm(): Promise<void> {
  closing.value = true;
  error.value = null;
  try {
    await closeSession(props.sessionId, notes.value.trim() || undefined);
    requesting.value = false;
    emit('closed');
  } catch (e) {
    error.value =
      e instanceof AdminApiError
        ? e.message
        : 'Impossibile terminare la sessione. Riprova più tardi.';
    requesting.value = false;
  } finally {
    closing.value = false;
  }
}
</script>

<template>
  <section class="close-session-form">
    <h2>Termina sessione</h2>

    <div class="form-group">
      <label for="session-notes">Note (facoltative)</label>
      <textarea
        id="session-notes"
        v-model="notes"
        class="form-control"
        rows="2"
      />
    </div>

    <DsButton
      variant="danger"
      outline
      :disabled="closing"
      @click="requestClose"
    >
      Termina sessione
    </DsButton>

    <DsCallout v-if="error" variant="danger" title="Errore">
      {{ error }}
    </DsCallout>

    <DsConfirmDialog
      data-testid="close-session-dialog"
      :open="requesting"
      message="Terminare questa sessione? Non potrà più essere riaperta né modificata."
      confirm-label="Termina sessione"
      @confirm="confirm"
      @cancel="cancel"
    />
  </section>
</template>
