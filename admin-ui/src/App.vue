<script setup lang="ts">
import { DsNav } from './components/ds';

const businessLinks = [
  { to: '/ingest', label: 'Ingest' },
  { to: '/imprinting', label: 'Imprinting' },
  { to: '/training', label: 'Training' },
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
