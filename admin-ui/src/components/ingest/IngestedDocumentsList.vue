<script setup lang="ts">
import { onMounted, ref } from 'vue';

import { DsButton, DsCallout } from '../ds';
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

function isLink(sourceRef: string): boolean {
  return /^https?:\/\//.test(sourceRef);
}

function sourceLabel(source: string): string {
  if (source === 'scrape') return 'Scraping';
  if (source === 'manual') return 'Caricamento manuale';
  return source;
}

async function load(): Promise<void> {
  loading.value = true;
  error.value = null;
  try {
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

      <ul v-else class="ingested-documents__list">
        <li v-for="doc in documents" :key="doc.source_ref">
          <a
            v-if="isLink(doc.source_ref)"
            :href="doc.source_ref"
            target="_blank"
            rel="noopener noreferrer"
          >
            {{ doc.source_ref }}
          </a>
          <span v-else>{{ doc.source_ref }}</span>
          <span class="badge badge-secondary">{{ sourceLabel(doc.source) }}</span>
          <span class="ingested-documents__count">
            {{ doc.chunk_count }} {{ doc.chunk_count === 1 ? 'blocco' : 'blocchi' }}
          </span>
        </li>
      </ul>
    </template>
  </section>
</template>

<style scoped>
.ingested-documents__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

.ingested-documents__list {
  list-style: none;
  padding: 0;
}

.ingested-documents__list li {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 0;
  border-bottom: 1px solid rgba(31, 42, 55, 0.08);
}

.ingested-documents__count {
  font-size: 0.85rem;
  color: var(--spontini-color-text-muted, #6c757d);
}
</style>
