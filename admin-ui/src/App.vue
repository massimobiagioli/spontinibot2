<script setup lang="ts">
import { DsNav } from './components/ds';

// Imprinting and Training (features 0017/0018) don't exist yet, so they
// remain disabled placeholders per plan 0015's non-goals.
const businessLinks = [
  { to: '/ingest', label: 'Ingest' },
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
