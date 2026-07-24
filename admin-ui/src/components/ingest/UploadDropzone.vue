<script setup lang="ts">
import { computed, ref, useId } from 'vue';

import { DsButton, DsInput } from '../ds';
import {
  AdminApiError,
  confirmUpload,
  getUploadPreview,
  uploadDocument,
  type PreviewResponse,
  type UploadMetadataInput,
} from '../../services/adminApi';

const props = defineProps<{
  sectionName: string;
}>();

type Phase = 'idle' | 'uploading' | 'preview' | 'confirming' | 'done';

const phase = ref<Phase>('idle');
const selectedFile = ref<File | null>(null);
const category = ref('');
const tags = ref('');
const trustScore = ref('');
const error = ref<string | null>(null);
const token = ref<string | null>(null);
const preview = ref<PreviewResponse | null>(null);
const chunkCount = ref<number | null>(null);

const fileInputId = useId();

const previewSummary = computed(() => {
  if (!preview.value) return '';
  const { filename, format, byte_size, chunk_count_estimate } = preview.value;
  return `${filename} (${format}, ${byte_size} byte, ~${chunk_count_estimate} blocchi)`;
});

function onFileChange(event: Event): void {
  const input = event.target as HTMLInputElement;
  selectedFile.value = input.files?.[0] ?? null;
}

function resetForm(): void {
  phase.value = 'idle';
  selectedFile.value = null;
  category.value = '';
  tags.value = '';
  trustScore.value = '';
  token.value = null;
  preview.value = null;
}

async function upload(): Promise<void> {
  if (!selectedFile.value) return;
  phase.value = 'uploading';
  error.value = null;
  try {
    const metadata: UploadMetadataInput = {};
    if (category.value) metadata.category = category.value;
    if (tags.value) metadata.tags = tags.value.split(',').map((t) => t.trim());
    if (trustScore.value) metadata.trustScore = Number(trustScore.value);

    const uploaded = await uploadDocument(
      selectedFile.value,
      props.sectionName,
      metadata,
    );
    token.value = uploaded.token;
    preview.value = await getUploadPreview(uploaded.token);
    phase.value = 'preview';
  } catch (e) {
    error.value =
      e instanceof AdminApiError
        ? e.message
        : 'Impossibile caricare il documento. Riprova più tardi.';
    phase.value = 'idle';
  }
}

async function confirm(): Promise<void> {
  if (!token.value) return;
  phase.value = 'confirming';
  error.value = null;
  try {
    const result = await confirmUpload(token.value);
    chunkCount.value = result.chunk_count;
    phase.value = 'done';
  } catch (e) {
    error.value =
      e instanceof AdminApiError
        ? e.message
        : 'Impossibile confermare il caricamento. Riprova più tardi.';
    phase.value = 'preview';
  }
}

function cancel(): void {
  resetForm();
}
</script>

<template>
  <div>
    <h3>Caricamento manuale</h3>

    <form
      v-if="phase === 'idle' || phase === 'uploading'"
      @submit.prevent="upload"
    >
      <div class="form-group">
        <label :for="fileInputId">File (pdf, docx, md, txt)</label>
        <input
          :id="fileInputId"
          type="file"
          accept=".pdf,.docx,.md,.txt"
          @change="onFileChange"
        />
      </div>
      <DsInput v-model="category" label="Categoria" />
      <DsInput v-model="tags" label="Tag (separati da virgola)" />
      <DsInput v-model="trustScore" label="Trust score" type="number" />

      <p v-if="error" role="alert">{{ error }}</p>

      <DsButton
        type="submit"
        :disabled="!selectedFile || phase === 'uploading'"
      >
        {{ phase === 'uploading' ? 'Caricamento…' : 'Carica' }}
      </DsButton>
    </form>

    <div v-else-if="phase === 'preview' || phase === 'confirming'">
      <p v-if="preview">{{ previewSummary }}</p>
      <p v-if="preview">{{ preview.extracted_text.slice(0, 500) }}</p>

      <p v-if="error" role="alert">{{ error }}</p>

      <DsButton :disabled="phase === 'confirming'" @click="confirm">
        {{ phase === 'confirming' ? 'Conferma in corso…' : 'Conferma' }}
      </DsButton>
      <DsButton variant="secondary" outline @click="cancel">Annulla</DsButton>
    </div>

    <div v-else-if="phase === 'done'">
      <p>Documento indicizzato: {{ chunkCount }} blocchi creati.</p>
      <DsButton @click="resetForm">Carica un altro documento</DsButton>
    </div>
  </div>
</template>
