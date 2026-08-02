<script setup lang="ts">
import { ref } from 'vue';

import {
  DsAccordion,
  DsButton,
  DsCallout,
  DsInfoButton,
  DsInput,
  DsNav,
  DsPagination,
} from '../components/ds';

const inputValue = ref('');
const inputWithHint = ref('');
const catalogPage = ref(2);
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
    <h2>DsInfoButton</h2>
    <p>
      Un pulsante "i" che apre una dialog con informazioni di supporto, pensato
      per affiancare l'etichetta di un campo (slot
      <code>label-suffix</code> di <code>DsInput</code>).
    </p>
    <DsInfoButton label="Informazioni di esempio" title="Titolo della dialog">
      Contenuto di esempio della dialog informativa.
    </DsInfoButton>
  </section>

  <section>
    <h2>DsAccordion</h2>
    <DsAccordion title="Primo pannello" default-open>
      <p>Contenuto del primo pannello, aperto di default.</p>
    </DsAccordion>
    <DsAccordion title="Secondo pannello">
      <p>Contenuto del secondo pannello, chiuso di default.</p>
    </DsAccordion>
  </section>

  <section>
    <h2>DsPagination</h2>
    <p>
      Componente unico di paginazione — usato da SessionList e QuestionGrid, non
      riscritto ad-hoc in ciascuna pagina.
    </p>
    <DsPagination
      v-model:current-page="catalogPage"
      :total-pages="5"
      label="Esempio di paginazione"
    />
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
