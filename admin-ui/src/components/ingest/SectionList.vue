<script setup lang="ts">
import { ref } from 'vue';

import { DsButton, DsConfirmDialog, DsInput } from '../ds';
import {
  AdminApiError,
  createSection,
  deleteSection,
  type IngestSectionWithSources,
} from '../../services/adminApi';
import SourceList from './SourceList.vue';
import UploadDropzone from './UploadDropzone.vue';

defineProps<{
  sections: IngestSectionWithSources[];
}>();

const emit = defineEmits<{ changed: [] }>();

const newSectionName = ref('');
const newSectionOrdering = ref(10);
const addError = ref<string | null>(null);
const adding = ref(false);

async function addSection(): Promise<void> {
  adding.value = true;
  addError.value = null;
  try {
    await createSection(newSectionName.value, newSectionOrdering.value);
    newSectionName.value = '';
    newSectionOrdering.value = 10;
    emit('changed');
  } catch (e) {
    addError.value =
      e instanceof AdminApiError
        ? e.message
        : 'Impossibile creare la sezione. Riprova più tardi.';
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
    await deleteSection(id);
    pendingDeleteId.value = null;
    emit('changed');
  } catch (e) {
    deleteError.value =
      e instanceof AdminApiError
        ? e.message
        : 'Impossibile eliminare la sezione. Riprova più tardi.';
    pendingDeleteId.value = null;
  }
}
</script>

<template>
  <section>
    <h2>Sezioni</h2>

    <p v-if="deleteError" role="alert">{{ deleteError }}</p>

    <ul>
      <li v-for="section in sections" :key="section.id">
        <span>{{ section.name }} (ordine {{ section.ordering }})</span>
        <DsButton variant="danger" outline @click="requestDelete(section.id)">
          Elimina sezione
        </DsButton>

        <SourceList
          :section-id="section.id"
          :sources="section.sources"
          @changed="emit('changed')"
        />

        <UploadDropzone :section-name="section.name" />
      </li>
    </ul>

    <form @submit.prevent="addSection">
      <h3>Aggiungi sezione</h3>
      <DsInput v-model="newSectionName" label="Nome sezione" required />
      <DsInput
        :model-value="String(newSectionOrdering)"
        label="Ordine"
        type="number"
        @update:model-value="newSectionOrdering = Number($event) || 0"
      />
      <p v-if="addError" role="alert">{{ addError }}</p>
      <DsButton type="submit" :disabled="adding">
        {{ adding ? 'Aggiunta…' : 'Aggiungi sezione' }}
      </DsButton>
    </form>

    <DsConfirmDialog
      data-testid="section-delete-dialog"
      :open="pendingDeleteId !== null"
      message="Eliminare questa sezione? Verranno rimosse anche le sue fonti."
      confirm-label="Elimina"
      @confirm="confirmDelete"
      @cancel="cancelDelete"
    />
  </section>
</template>
