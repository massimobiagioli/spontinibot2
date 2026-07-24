<script setup lang="ts">
import { ref } from 'vue';

import { DsConfirmDialog } from '../ds';
import {
  AdminApiError,
  activatePersona,
  type PersonaResponse,
} from '../../services/adminApi';

defineProps<{
  versions: PersonaResponse[];
}>();

const emit = defineEmits<{ changed: [] }>();

const pendingActivateId = ref<number | null>(null);
const error = ref<string | null>(null);

function requestActivate(id: number): void {
  error.value = null;
  pendingActivateId.value = id;
}

function cancelActivate(): void {
  pendingActivateId.value = null;
}

async function confirmActivate(): Promise<void> {
  const id = pendingActivateId.value;
  if (id === null) return;
  try {
    await activatePersona(id);
    pendingActivateId.value = null;
    emit('changed');
  } catch (e) {
    error.value =
      e instanceof AdminApiError
        ? e.message
        : 'Impossibile attivare questa versione. Riprova più tardi.';
    pendingActivateId.value = null;
  }
}
</script>

<template>
  <section>
    <h2>Cronologia versioni</h2>

    <p v-if="error" role="alert">{{ error }}</p>

    <ul>
      <li v-for="version in versions" :key="version.id">
        <span>v{{ version.version }} — {{ version.created_at }}</span>
        <span v-if="version.created_by"> — {{ version.created_by }}</span>
        <span v-if="version.is_active" class="badge badge-success">
          Attiva
        </span>
        <p>{{ version.system_prompt.slice(0, 120) }}</p>
        <button
          v-if="!version.is_active"
          type="button"
          class="btn btn-outline-primary"
          @click="requestActivate(version.id)"
        >
          Attiva questa versione
        </button>
      </li>
    </ul>

    <DsConfirmDialog
      data-testid="activate-dialog"
      :open="pendingActivateId !== null"
      message="Attivare questa versione? Diventerà immediatamente quella usata da /chat e dalle sessioni di training."
      confirm-label="Attiva"
      @confirm="confirmActivate"
      @cancel="cancelActivate"
    />
  </section>
</template>
