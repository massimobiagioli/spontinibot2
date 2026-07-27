<script setup lang="ts">
import { onMounted, ref } from 'vue';

import { DsButton } from '../ds';
import type { IngestedDocumentResponse } from '../../services/adminApi';

const props = defineProps<{
  document: IngestedDocumentResponse;
}>();

const emit = defineEmits<{ close: [] }>();

const dialogRef = ref<HTMLDialogElement | null>(null);

function isLink(sourceRef: string): boolean {
  return /^https?:\/\//.test(sourceRef);
}

function sourceLabel(source: string): string {
  if (source === 'scrape') return 'Scraping';
  if (source === 'manual') return 'Caricamento manuale';
  return source;
}

onMounted(() => {
  if (typeof dialogRef.value?.showModal === 'function') {
    dialogRef.value.showModal();
  }
});

function close(): void {
  emit('close');
}
</script>

<template>
  <dialog ref="dialogRef" class="document-detail" @cancel.prevent="close">
    <div class="document-detail__content">
      <header class="document-detail__header">
        <h2>Scheda contenuto</h2>
        <button
          type="button"
          class="document-detail__close-icon"
          aria-label="Chiudi"
          @click="close"
        >
          &times;
        </button>
      </header>

      <div class="document-detail__body">
        <section class="document-detail__field">
          <p class="document-detail__label">Fonte</p>
          <p class="document-detail__value">
            <a
              v-if="isLink(props.document.source_ref)"
              :href="props.document.source_ref"
              target="_blank"
              rel="noopener noreferrer"
            >
              {{ props.document.source_ref }}
            </a>
            <span v-else>{{ props.document.source_ref }}</span>
          </p>
        </section>

        <dl class="document-detail__meta">
          <div class="document-detail__meta-item">
            <dt>Origine</dt>
            <dd>{{ sourceLabel(props.document.source) }}</dd>
          </div>
          <div class="document-detail__meta-item">
            <dt>Blocchi</dt>
            <dd>{{ props.document.chunk_count }}</dd>
          </div>
          <div class="document-detail__meta-item">
            <dt>Ingerito il</dt>
            <dd>{{ props.document.created_at }}</dd>
          </div>
        </dl>
      </div>

      <footer class="document-detail__footer">
        <DsButton variant="secondary" outline @click="close">Chiudi</DsButton>
      </footer>
    </div>
  </dialog>
</template>

<style scoped>
.document-detail {
  border: none;
  border-radius: 8px;
  padding: 0;
  width: min(32rem, 90vw);
  max-height: 85vh;
  overflow: hidden;
}

.document-detail::backdrop {
  background: rgba(0, 0, 0, 0.5);
}

.document-detail__content {
  display: flex;
  flex-direction: column;
  max-height: 85vh;
}

.document-detail__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 1.25rem 1.5rem;
  border-bottom: 1px solid rgba(31, 42, 55, 0.08);
}

.document-detail__header h2 {
  margin: 0;
  font-size: 1.25rem;
}

.document-detail__close-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 44px;
  min-width: 44px;
  border: none;
  border-radius: 4px;
  background: none;
  font-size: 1.5rem;
  line-height: 1;
  color: var(--spontini-color-text-muted, #6c757d);
  cursor: pointer;
}

.document-detail__close-icon:hover,
.document-detail__close-icon:focus-visible {
  background: rgba(31, 42, 55, 0.06);
  color: var(--spontini-color-text, #1f2a37);
}

.document-detail__body {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
  padding: 1.5rem;
  overflow-y: auto;
  word-break: break-word;
}

.document-detail__field {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.document-detail__label {
  margin: 0;
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--spontini-color-text-muted, #6c757d);
}

.document-detail__value {
  margin: 0;
  line-height: 1.5;
}

.document-detail__meta {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  margin: 0;
  font-size: 0.9rem;
}

.document-detail__meta-item {
  display: flex;
  gap: 0.5rem;
}

.document-detail__meta-item dt {
  font-weight: 600;
  min-width: 7rem;
}

.document-detail__meta-item dd {
  margin: 0;
  color: var(--spontini-color-text-muted, #6c757d);
}

.document-detail__footer {
  display: flex;
  justify-content: flex-end;
  padding: 1rem 1.5rem;
  border-top: 1px solid rgba(31, 42, 55, 0.08);
}
</style>
