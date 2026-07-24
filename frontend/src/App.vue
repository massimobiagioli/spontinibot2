<script setup lang="ts"></script>

<template>
  <div class="app-shell">
    <header class="app-shell__header">
      <RouterLink to="/dev" class="app-shell__dev-link"
        >Catalogo componenti</RouterLink
      >
    </header>
    <main class="app-shell__main">
      <RouterView />
    </main>
  </div>
</template>
