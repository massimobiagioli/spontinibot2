<script setup lang="ts">
import { ref } from 'vue';

import { DsButton, DsCallout, DsInput } from '../components/ds';
import ChatInput from '../components/chat/ChatInput.vue';
import ChatMessage from '../components/chat/ChatMessage.vue';
import type { ChatResponse } from '../services/chatApi';

const inputValue = ref('');
const inputWithHint = ref('');

const answeredExample: ChatResponse = {
  answer: "Lo sportello anagrafe e' aperto dal lunedi' al venerdi'.",
  sources: [{ document_id: 1, source_ref: 'Orari sportello anagrafe' }],
  fell_back: false,
};

const fallbackExample: ChatResponse = {
  answer: 'Non ho trovato informazioni su questo argomento.',
  sources: [],
  fell_back: true,
};
</script>

<template>
  <h1>Catalogo componenti</h1>
  <p>Ogni wrapper DSI in isolamento, con le sue varianti principali.</p>

  <section>
    <h2>DsButton</h2>
    <DsButton variant="primary">Invia</DsButton>
    <DsButton variant="secondary">Secondario</DsButton>
    <DsButton variant="danger" outline>Contorno</DsButton>
    <DsButton variant="primary" disabled>Disabilitato</DsButton>
  </section>

  <section>
    <h2>DsInput</h2>
    <DsInput v-model="inputValue" label="Domanda" />
    <DsInput
      v-model="inputWithHint"
      label="Domanda con suggerimento"
      hint="Scrivi la tua domanda al Comune."
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
    <h2>ChatMessage</h2>
    <ChatMessage
      question="A che ore apre l'anagrafe?"
      :response="answeredExample"
    />
    <ChatMessage
      question="Quanto pago di tasse comunali?"
      :response="fallbackExample"
    />
  </section>

  <section>
    <h2>ChatInput</h2>
    <ChatInput :busy="false" />
    <ChatInput :busy="true" />
  </section>
</template>
