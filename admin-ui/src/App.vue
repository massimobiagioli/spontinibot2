<script setup lang="ts">
import { DsNav } from './components/ds';

// Milestone 3 business sections (features 0016-0018) don't exist yet, so
// these render as disabled placeholders per plan 0015's non-goals.
const businessLinks = [
  { label: 'Ingest' },
  { label: 'Imprinting' },
  { label: 'Training' },
];
</script>

<template>
  <div class="app-shell">
    <DsNav class="app-shell__nav" :links="businessLinks" />
    <main class="app-shell__main">
      <p><RouterLink to="/dev">Catalogo componenti</RouterLink></p>
      <RouterView />
    </main>
  </div>
</template>
