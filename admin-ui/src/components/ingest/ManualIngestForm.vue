<script setup lang="ts">
import { ref } from 'vue';

import { DsButton, DsCallout, DsInput } from '../ds';
import {
  AdminApiError,
  triggerManualIngest,
  type IngestManualResponse,
} from '../../services/adminApi';

const section = ref('');
const src = ref('');
const window_ = ref('');
const submitting = ref(false);
const result = ref<IngestManualResponse | null>(null);
const error = ref<string | null>(null);

async function submit(): Promise<void> {
  submitting.value = true;
  error.value = null;
  result.value = null;
  try {
    result.value = await triggerManualIngest(
      section.value,
      src.value,
      window_.value,
    );
  } catch (e) {
    error.value =
      e instanceof AdminApiError
        ? e.message
        : "Impossibile avviare l'ingest manuale. Riprova più tardi.";
  } finally {
    submitting.value = false;
  }
}
</script>

<template>
  <form @submit.prevent="submit">
    <DsInput
      v-model="section"
      label="Sezione"
      hint="es. storia"
      required
    />
    <DsInput
      v-model="src"
      label="Fonte (URL)"
      hint="deve consentire lo scraping (robots.txt)"
      required
    />
    <DsInput
      v-model="window_"
      label="Finestra temporale"
      hint="es. 30d oppure 2026-07"
      required
    />
    <DsButton
      type="submit"
      :disabled="submitting || !section || !src || !window_"
    >
      {{ submitting ? 'Avvio…' : 'Esegui ingest manuale' }}
    </DsButton>

    <DsCallout v-if="error" variant="danger" title="Errore">
      {{ error }}
    </DsCallout>

    <DsCallout v-else-if="result" variant="success" title="Ingest completato">
      Sezione "{{ result.section }}" aggiornata da {{ result.src }} (finestra
      {{ result.window }}).
    </DsCallout>
  </form>
</template>
