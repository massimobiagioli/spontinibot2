<script setup lang="ts">
import { onMounted, ref } from 'vue';

import { DsButton, DsCallout } from '../ds';
import {
  AdminApiError,
  listRobotsBypassHosts,
  replaceRobotsBypassHosts,
} from '../../services/adminApi';

const hostsText = ref('');
const loading = ref(true);
const loadError = ref<string | null>(null);
const saving = ref(false);
const saveError = ref<string | null>(null);
const saved = ref(false);

async function load(): Promise<void> {
  loading.value = true;
  loadError.value = null;
  try {
    const hosts = await listRobotsBypassHosts();
    hostsText.value = hosts.map((h) => h.host).join('\n');
  } catch (e) {
    loadError.value =
      e instanceof AdminApiError
        ? e.message
        : 'Impossibile contattare il server. Riprova più tardi.';
  } finally {
    loading.value = false;
  }
}

onMounted(load);

async function save(): Promise<void> {
  saving.value = true;
  saveError.value = null;
  saved.value = false;
  try {
    const hosts = await replaceRobotsBypassHosts(hostsText.value);
    hostsText.value = hosts.map((h) => h.host).join('\n');
    saved.value = true;
  } catch (e) {
    saveError.value =
      e instanceof AdminApiError
        ? e.message
        : 'Impossibile salvare. Riprova più tardi.';
  } finally {
    saving.value = false;
  }
}

function onEdit(): void {
  saved.value = false;
}
</script>

<template>
  <section>
    <h2>Scraper</h2>
    <p>
      Elenco dei siti autorizzati a bypassare completamente il controllo
      <code>robots.txt</code> durante l'ingestione — un'eccezione per ogni riga
      (es. <code>www.comune.maiolatispontini.an.it</code>). Ogni ingestione
      legge questo elenco al momento della richiesta: una modifica salvata qui
      ha effetto immediato, senza bisogno di riavviare nulla.
    </p>

    <p v-if="loading">Caricamento…</p>

    <DsCallout
      v-else-if="loadError"
      variant="danger"
      title="Impossibile caricare l'elenco"
    >
      {{ loadError }}
    </DsCallout>

    <form v-else @submit.prevent="save">
      <div class="form-group">
        <label for="robots-bypass-hosts">Siti eccezione</label>
        <textarea
          id="robots-bypass-hosts"
          v-model="hostsText"
          class="form-control"
          rows="8"
          placeholder="un-sito.esempio.it"
          @input="onEdit"
        />
      </div>

      <p v-if="saveError" role="alert">{{ saveError }}</p>
      <DsCallout v-if="saved" variant="success">Elenco salvato.</DsCallout>

      <DsButton type="submit" :disabled="saving">
        {{ saving ? 'Salvataggio…' : 'Salva' }}
      </DsButton>
    </form>
  </section>
</template>
