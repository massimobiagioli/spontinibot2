<script setup lang="ts">
import { ref } from 'vue';

import { DsConfirmDialog } from '../ds';
import {
  AdminApiError,
  activatePersona,
  deletePersonaVersion,
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
    await deletePersonaVersion(id);
    pendingDeleteId.value = null;
    emit('changed');
  } catch (e) {
    deleteError.value =
      e instanceof AdminApiError
        ? e.message
        : 'Impossibile eliminare questa versione. Riprova più tardi.';
    pendingDeleteId.value = null;
  }
}
</script>

<template>
  <section>
    <h2>Cronologia versioni</h2>

    <p v-if="error" role="alert">{{ error }}</p>
    <p v-if="deleteError" role="alert">{{ deleteError }}</p>

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
        <button
          v-if="!version.is_active"
          type="button"
          class="btn btn-outline-danger"
          @click="requestDelete(version.id)"
        >
          Elimina versione
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

    <DsConfirmDialog
      data-testid="delete-dialog"
      :open="pendingDeleteId !== null"
      message="Eliminare questa versione dalla cronologia? L'operazione non può essere annullata."
      confirm-label="Elimina"
      @confirm="confirmDelete"
      @cancel="cancelDelete"
    />
  </section>
</template>
