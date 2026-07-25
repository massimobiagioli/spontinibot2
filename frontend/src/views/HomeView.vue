<script setup lang="ts">
import ChatWidget from '../components/chat/ChatWidget.vue';
</script>

<template>
  <section class="home__hero">
    <h1 class="home__hero-title">Spontini Bot</h1>
    <p class="home__hero-subtitle">
      L'assistente digitale del Comune di Maiolati Spontini. Rispondiamo alle
      tue domande sui servizi comunali citando sempre la fonte ufficiale.
    </p>
  </section>

  <section class="home__cards">
    <article class="home__card">
      <h2>Chi è Spontini Bot</h2>
      <p>
        Un assistente virtuale che consulta i documenti pubblicati dal Comune
        per rispondere alle domande dei cittadini, 24 ore su 24.
      </p>
    </article>
    <article class="home__card">
      <h2>Come funziona</h2>
      <p>
        Apri la chat dall'icona in basso a destra, scrivi la tua domanda e
        ricevi una risposta basata sui documenti ufficiali del Comune.
      </p>
    </article>
    <article class="home__card">
      <h2>Fonti verificate</h2>
      <p>
        Ogni risposta indica i documenti da cui proviene. Se l'informazione non
        è disponibile, Spontini Bot lo dice onestamente.
      </p>
    </article>
  </section>

  <ChatWidget />
</template>
