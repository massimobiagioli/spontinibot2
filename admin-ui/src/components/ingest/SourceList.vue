<script setup lang="ts">
import { ref } from 'vue';

import { DsButton, DsConfirmDialog, DsInput } from '../ds';
import {
  AdminApiError,
  createSource,
  deleteSource,
  type CurationSourceResponse,
  type IngestSourceResponse,
} from '../../services/adminApi';

const props = defineProps<{
  sectionId: number;
  sources: IngestSourceResponse[];
  curationSources: CurationSourceResponse[];
}>();

const emit = defineEmits<{ changed: [] }>();

const newSourceUrl = ref('');
const addError = ref<string | null>(null);
const adding = ref(false);

async function addSource(): Promise<void> {
  adding.value = true;
  addError.value = null;
  try {
    await createSource(props.sectionId, 'scrape', newSourceUrl.value, true);
    newSourceUrl.value = '';
    emit('changed');
  } catch (e) {
    addError.value =
      e instanceof AdminApiError
        ? e.message
        : 'Impossibile creare la fonte. Riprova più tardi.';
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
    await deleteSource(id);
    pendingDeleteId.value = null;
    emit('changed');
  } catch (e) {
    deleteError.value =
      e instanceof AdminApiError
        ? e.message
        : 'Impossibile eliminare la fonte. Riprova più tardi.';
    pendingDeleteId.value = null;
  }
}
</script>

<template>
  <div>
    <h3>Fonti</h3>

    <p v-if="deleteError" role="alert">{{ deleteError }}</p>

    <p
      v-if="sources.length === 0 && curationSources.length === 0"
      class="source-list__empty"
    >
      Nessuna fonte configurata per questa sezione.
    </p>

    <ul v-if="curationSources.length > 0">
      <li
        v-for="curation in curationSources"
        :key="curation.source_url"
        class="source-list__curation-item"
      >
        <span class="badge badge-secondary">Curazione automatica</span>
        <span>{{ curation.source_url }}</span>
        <span class="source-list__curation-date">
          ultimo aggiornamento: {{ curation.last_item_date }}
        </span>
      </li>
    </ul>

    <ul v-if="sources.length > 0">
      <li
        v-for="source in sources"
        :key="source.id"
        :class="{ 'text-muted': source.coming_soon }"
      >
        <span
          v-if="source.coming_soon"
          title="Le fonti di tipo API arriveranno in un prossimo aggiornamento"
        >
          {{ source.url }} (API — Prossimamente)
        </span>
        <span v-else>{{ source.url }} ({{ source.source_type }})</span>

        <DsButton variant="danger" outline @click="requestDelete(source.id)">
          Elimina fonte
        </DsButton>
      </li>
    </ul>

    <form @submit.prevent="addSource">
      <DsInput v-model="newSourceUrl" label="URL fonte scrape" required />
      <p v-if="addError" role="alert">{{ addError }}</p>
      <DsButton type="submit" :disabled="adding">
        {{ adding ? 'Aggiunta…' : 'Aggiungi fonte' }}
      </DsButton>
    </form>

    <DsConfirmDialog
      :open="pendingDeleteId !== null"
      message="Eliminare questa fonte?"
      confirm-label="Elimina"
      @confirm="confirmDelete"
      @cancel="cancelDelete"
    />
  </div>
</template>

<style scoped>
.source-list__curation-item {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.5rem;
}

.source-list__curation-date {
  font-size: 0.85rem;
  color: var(--spontini-color-text-muted, #6c757d);
}
</style>
