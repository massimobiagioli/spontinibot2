<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';

import { DsButton, DsCallout } from '../ds';
import DocumentDetail from './DocumentDetail.vue';
import {
  AdminApiError,
  listSectionDocuments,
  type IngestedDocumentResponse,
} from '../../services/adminApi';

const props = defineProps<{
  sectionId: number;
}>();

const documents = ref<IngestedDocumentResponse[] | null>(null);
const loading = ref(true);
const error = ref<string | null>(null);
const selectedRef = ref<string | null>(null);

function sourceLabel(source: string): string {
  if (source === 'scrape') return 'Scraping';
  if (source === 'manual') return 'Caricamento manuale';
  return source;
}

const selectedDocument = computed(
  () => documents.value?.find((d) => d.source_ref === selectedRef.value) ?? null,
);

function open(sourceRef: string): void {
  selectedRef.value = sourceRef;
}

function close(): void {
  selectedRef.value = null;
}

async function load(): Promise<void> {
  loading.value = true;
  error.value = null;
  try {
    // The backend already returns these newest-first (ORDER BY created_at
    // DESC) — no client-side sort needed.
    documents.value = await listSectionDocuments(props.sectionId);
  } catch (e) {
    error.value =
      e instanceof AdminApiError
        ? e.message
        : 'Impossibile contattare il server. Riprova più tardi.';
  } finally {
    loading.value = false;
  }
}

onMounted(load);

defineExpose({ load });
</script>

<template>
  <section>
    <div class="ingested-documents__header">
      <h2>Contenuti ingeriti</h2>
      <DsButton variant="secondary" outline :disabled="loading" @click="load">
        Aggiorna
      </DsButton>
    </div>

    <p v-if="loading">Caricamento dei contenuti ingeriti…</p>

    <DsCallout
      v-else-if="error"
      variant="danger"
      title="Impossibile caricare i contenuti ingeriti"
    >
      {{ error }}
    </DsCallout>

    <template v-else-if="documents">
      <p v-if="documents.length === 0">
        Nessun contenuto ingerito per questa sezione.
      </p>

      <div v-else class="ingested-documents__grid">
        <button
          v-for="doc in documents"
          :key="doc.source_ref"
          type="button"
          class="ingested-documents__card"
          @click="open(doc.source_ref)"
        >
          <p class="ingested-documents__ref">{{ doc.source_ref }}</p>
          <div class="ingested-documents__meta">
            <span class="badge badge-secondary">{{ sourceLabel(doc.source) }}</span>
            <span class="ingested-documents__count">
              {{ doc.chunk_count }} {{ doc.chunk_count === 1 ? 'blocco' : 'blocchi' }}
            </span>
            <span class="ingested-documents__date">{{ doc.created_at }}</span>
          </div>
        </button>
      </div>
    </template>

    <DocumentDetail
      v-if="selectedDocument"
      :document="selectedDocument"
      @close="close"
    />
  </section>
</template>

<style scoped>
.ingested-documents__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

.ingested-documents__grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
  gap: 1rem;
}

.ingested-documents__card {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.5rem;
  text-align: left;
  padding: 1rem;
  border: 1px solid rgba(31, 42, 55, 0.12);
  border-radius: 8px;
  background: var(--spontini-color-white, #fff);
  cursor: pointer;
}

.ingested-documents__card:hover,
.ingested-documents__card:focus-visible {
  border-color: var(--spontini-color-primary);
}

.ingested-documents__ref {
  font-weight: 600;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  margin: 0;
  word-break: break-word;
}

.ingested-documents__meta {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  align-items: center;
}

.ingested-documents__count,
.ingested-documents__date {
  font-size: 0.85rem;
  color: var(--spontini-color-text-muted, #6c757d);
}
</style>
