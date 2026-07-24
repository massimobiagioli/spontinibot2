<script setup lang="ts">
import { onMounted, ref } from 'vue';

import { DsCallout } from '../components/ds';
import RunTrigger from '../components/ingest/RunTrigger.vue';
import ScheduleEditor from '../components/ingest/ScheduleEditor.vue';
import SectionList from '../components/ingest/SectionList.vue';
import {
  AdminApiError,
  getIngestConfig,
  type IngestConfigResponse,
  type IngestScheduleResponse,
} from '../services/adminApi';

const config = ref<IngestConfigResponse | null>(null);
const loading = ref(true);
const error = ref<string | null>(null);

async function loadConfig(): Promise<void> {
  loading.value = true;
  error.value = null;
  try {
    config.value = await getIngestConfig();
  } catch (e) {
    error.value =
      e instanceof AdminApiError
        ? e.message
        : 'Impossibile contattare il server. Riprova più tardi.';
  } finally {
    loading.value = false;
  }
}

onMounted(loadConfig);

function onScheduleSaved(schedule: IngestScheduleResponse): void {
  if (config.value) config.value.schedule = schedule;
}
</script>

<template>
  <h1>Configurazione ingest</h1>

  <p v-if="loading">Caricamento della configurazione…</p>

  <DsCallout
    v-else-if="error"
    variant="danger"
    title="Impossibile caricare la configurazione"
  >
    {{ error }}
  </DsCallout>

  <template v-else-if="config">
    <RunTrigger />
    <ScheduleEditor :schedule="config.schedule" @saved="onScheduleSaved" />
    <SectionList :sections="config.sections" @changed="loadConfig" />
  </template>
</template>
