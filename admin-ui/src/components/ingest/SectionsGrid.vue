<script setup lang="ts">
import { computed, ref } from 'vue';

import { DsButton, DsCallout, DsInput } from '../ds';
import {
  AdminApiError,
  createSection,
  type IngestSectionWithSources,
} from '../../services/adminApi';

const props = defineProps<{
  sections: IngestSectionWithSources[];
}>();

const emit = defineEmits<{ changed: [] }>();

function sourceCountLabel(section: IngestSectionWithSources): string {
  const n = section.sources.length + section.curation_sources.length;
  return n === 1 ? '1 fonte configurata' : `${n} fonti configurate`;
}

const showAddForm = ref(false);
const newSectionName = ref('');
const addError = ref<string | null>(null);
const adding = ref(false);

// The operator never sees or sets an ordering number — it's purely an
// internal sort key. Auto-derived as the next slot after the highest
// existing one, mirroring the increment-by-10 convention already used by
// every hand-authored section in this project (see ROADMAP.md feature 0016).
const nextOrdering = computed(() => {
  const max = props.sections.reduce((m, s) => Math.max(m, s.ordering), 0);
  return max + 10;
});

async function addSection(): Promise<void> {
  adding.value = true;
  addError.value = null;
  try {
    await createSection(newSectionName.value, nextOrdering.value);
    newSectionName.value = '';
    showAddForm.value = false;
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
</script>

<template>
  <section>
    <div class="sections-grid__header">
      <DsButton v-if="!showAddForm" @click="showAddForm = true">
        Aggiungi sezione
      </DsButton>
    </div>

    <DsCallout
      v-if="sections.length === 0 && !showAddForm"
      variant="primary"
      title="Nessuna sezione"
    >
      Non esiste ancora nessuna sezione di ingest. Aggiungine una con il
      pulsante "Aggiungi sezione".
    </DsCallout>

    <div v-if="showAddForm" class="sections-grid__add-form">
      <form @submit.prevent="addSection">
        <DsInput v-model="newSectionName" label="Nome sezione" required />
        <p v-if="addError" role="alert">{{ addError }}</p>
        <div class="sections-grid__add-actions">
          <DsButton type="submit" :disabled="adding">
            {{ adding ? 'Aggiunta…' : 'Salva' }}
          </DsButton>
          <DsButton
            variant="secondary"
            outline
            type="button"
            @click="showAddForm = false"
          >
            Annulla
          </DsButton>
        </div>
      </form>
    </div>

    <div class="row">
      <div
        v-for="section in sections"
        :key="section.id"
        class="col-12 col-sm-6 col-lg-4 sections-grid__col"
      >
        <RouterLink
          :to="`/ingest/sections/${section.id}`"
          class="it-card clickable-card sections-grid__card"
        >
          <div class="it-card-title">
            <h3>{{ section.name }}</h3>
          </div>
          <dl class="it-card-description-list">
            <dt>Fonti</dt>
            <dd>{{ sourceCountLabel(section) }}</dd>
          </dl>
        </RouterLink>
      </div>
    </div>
  </section>
</template>

<style scoped>
.sections-grid__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  margin-bottom: 1rem;
}

.sections-grid__add-form {
  margin-bottom: 1.5rem;
}

.sections-grid__add-actions {
  display: flex;
  gap: 0.75rem;
}

.sections-grid__col {
  margin-bottom: 1.5rem;
}

.sections-grid__card {
  display: block;
  height: 100%;
  text-decoration: none;
  color: inherit;
}
</style>
