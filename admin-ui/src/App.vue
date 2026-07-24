<script setup lang="ts">
import { DsNav } from './components/ds';

// Training (feature 0018) doesn't exist yet, so it remains a disabled
// placeholder per plan 0015's non-goals.
const businessLinks = [
  { to: '/ingest', label: 'Ingest' },
  { to: '/imprinting', label: 'Imprinting' },
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
