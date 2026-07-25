<script setup lang="ts"></script>

<template>
  <div class="app-shell">
    <header class="app-shell__header">
      <span class="app-shell__brand">
        <span aria-hidden="true">🎼</span>
        Comune di Maiolati Spontini
      </span>
      <RouterLink to="/dev" class="app-shell__dev-link"
        >Catalogo componenti</RouterLink
      >
    </header>
    <main class="app-shell__main">
      <RouterView />
    </main>
    <footer class="app-shell__footer">
      Comune di Maiolati Spontini — Spontini Bot
    </footer>
  </div>
</template>
