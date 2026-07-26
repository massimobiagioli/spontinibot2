<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';

import { DsCallout } from '../components/ds';
import PersonaEditor from '../components/imprinting/PersonaEditor.vue';
import ReloadPersonaButton from '../components/imprinting/ReloadPersonaButton.vue';
import VersionHistory from '../components/imprinting/VersionHistory.vue';
import {
  AdminApiError,
  getPersonaVersions,
  type PersonaResponse,
} from '../services/adminApi';

const personaName = ref(import.meta.env.VITE_PERSONA_NAME ?? 'Gaspare');

const versions = ref<PersonaResponse[] | null>(null);
const loading = ref(true);
const error = ref<string | null>(null);

const activeVersion = computed(
  () => versions.value?.find((v) => v.is_active) ?? null,
);

async function loadVersions(): Promise<void> {
  loading.value = true;
  error.value = null;
  try {
    versions.value = await getPersonaVersions(personaName.value);
  } catch (e) {
    error.value =
      e instanceof AdminApiError
        ? e.message
        : 'Impossibile contattare il server. Riprova più tardi.';
  } finally {
    loading.value = false;
  }
}

onMounted(loadVersions);

function onPersonaSaved(persona: PersonaResponse): void {
  // The name field is editable, so a save can rename the persona — track
  // the new name so the next fetch (and VersionHistory) follow it.
  personaName.value = persona.name;
  void loadVersions();
}

function onVersionsChanged(): void {
  void loadVersions();
}
</script>

<template>
  <h1>Imprinting del bot</h1>

  <ReloadPersonaButton />

  <p v-if="loading">Caricamento della persona…</p>

  <DsCallout
    v-else-if="error"
    variant="danger"
    title="Impossibile caricare la persona"
  >
    {{ error }}
  </DsCallout>

  <template v-else-if="versions">
    <DsCallout
      v-if="versions.length === 0"
      variant="primary"
      title="Nessuna persona configurata"
    >
      Non esiste ancora nessuna versione della persona "{{ personaName }}".
      Salva la prima versione qui sotto.
    </DsCallout>
    <p v-else>
      Versione attiva:
      {{ activeVersion ? `v${activeVersion.version}` : 'nessuna' }} ({{
        versions.length
      }}
      version{{ versions.length === 1 ? 'e' : 'i' }} totali)
    </p>

    <PersonaEditor
      :persona-name="personaName"
      :active-version="activeVersion"
      @saved="onPersonaSaved"
    />

    <VersionHistory
      v-if="versions.length > 0"
      :versions="versions"
      @changed="onVersionsChanged"
    />
  </template>
</template>
