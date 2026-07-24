<script setup lang="ts">
import { ref } from 'vue';

import { DsButton, DsCallout, DsInput, DsNav } from '../components/ds';

const inputValue = ref('');
const inputWithHint = ref('');
</script>

<template>
  <h1>Catalogo componenti</h1>
  <p>Ogni wrapper DSI in isolamento, con le sue varianti principali.</p>

  <section>
    <h2>DsButton</h2>
    <DsButton variant="primary">Primario</DsButton>
    <DsButton variant="secondary">Secondario</DsButton>
    <DsButton variant="danger" outline>Contorno</DsButton>
    <DsButton variant="primary" disabled>Disabilitato</DsButton>
  </section>

  <section>
    <h2>DsInput</h2>
    <DsInput v-model="inputValue" label="Nome" />
    <DsInput
      v-model="inputWithHint"
      label="Email"
      hint="Verrà usata solo per contatti di servizio."
    />
    <DsInput model-value="" label="Campo disabilitato" disabled />
  </section>

  <section>
    <h2>DsCallout</h2>
    <DsCallout variant="primary" title="Informazione"
      >Un callout informativo standard.</DsCallout
    >
    <DsCallout variant="warning" title="Attenzione" highlight
      >Un callout di avviso, in evidenza.</DsCallout
    >
    <DsCallout variant="danger">Un callout di errore senza titolo.</DsCallout>
  </section>

  <section>
    <h2>DsNav</h2>
    <DsNav
      aria-label="Esempio di navigazione"
      :links="[
        { to: '/', label: 'Home' },
        { to: '/dev', label: 'Catalogo componenti' },
        { label: 'Sezione futura' },
      ]"
    />
  </section>
</template>
