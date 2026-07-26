<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';

import { DsButton, DsCallout, DsConfirmDialog } from '../components/ds';
import SourceList from '../components/ingest/SourceList.vue';
import UploadDropzone from '../components/ingest/UploadDropzone.vue';
import {
  AdminApiError,
  getIngestConfig,
  deleteSection,
  type IngestSectionWithSources,
} from '../services/adminApi';

const route = useRoute();
const router = useRouter();
const sectionId = computed(() => Number(route.params['id']));

const section = ref<IngestSectionWithSources | null>(null);
const loading = ref(true);
const error = ref<string | null>(null);
const notFound = ref(false);

async function loadSection(): Promise<void> {
  loading.value = true;
  error.value = null;
  notFound.value = false;
  try {
    const config = await getIngestConfig();
    const found = config.sections.find((s) => s.id === sectionId.value);
    if (!found) {
      notFound.value = true;
      section.value = null;
    } else {
      section.value = found;
    }
  } catch (e) {
    error.value =
      e instanceof AdminApiError
        ? e.message
        : 'Impossibile contattare il server. Riprova più tardi.';
  } finally {
    loading.value = false;
  }
}

onMounted(loadSection);

const pendingDelete = ref(false);
const deleteError = ref<string | null>(null);

function requestDelete(): void {
  deleteError.value = null;
  pendingDelete.value = true;
}

function cancelDelete(): void {
  pendingDelete.value = false;
}

async function confirmDelete(): Promise<void> {
  if (!section.value) return;
  try {
    await deleteSection(section.value.id);
    pendingDelete.value = false;
    await router.push({ name: 'ingest' });
  } catch (e) {
    deleteError.value =
      e instanceof AdminApiError
        ? e.message
        : 'Impossibile eliminare la sezione. Riprova più tardi.';
    pendingDelete.value = false;
  }
}
</script>

<template>
  <p v-if="loading">Caricamento della sezione…</p>

  <DsCallout
    v-else-if="error"
    variant="danger"
    title="Impossibile caricare la sezione"
  >
    {{ error }}
  </DsCallout>

  <DsCallout v-else-if="notFound" variant="warning" title="Sezione non trovata">
    Questa sezione non esiste più.
    <RouterLink to="/ingest">Torna alle sezioni</RouterLink>.
  </DsCallout>

  <template v-else-if="section">
    <RouterLink to="/ingest" class="section-detail__back">
      &larr; Torna alle sezioni
    </RouterLink>

    <div class="section-detail__header">
      <h1>{{ section.name }}</h1>
      <DsButton
        variant="danger"
        outline
        data-testid="delete-section-button"
        @click="requestDelete"
      >
        Elimina sezione
      </DsButton>
    </div>

    <p v-if="deleteError" role="alert">{{ deleteError }}</p>

    <SourceList
      :section-id="section.id"
      :sources="section.sources"
      @changed="loadSection"
    />

    <UploadDropzone :section-name="section.name" />

    <DsConfirmDialog
      data-testid="section-delete-dialog"
      :open="pendingDelete"
      message="Eliminare questa sezione? Verranno rimosse anche le sue fonti."
      confirm-label="Elimina"
      @confirm="confirmDelete"
      @cancel="cancelDelete"
    />
  </template>
</template>

<style scoped>
.section-detail__back {
  display: inline-block;
  margin-bottom: 1rem;
}

.section-detail__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}
</style>
