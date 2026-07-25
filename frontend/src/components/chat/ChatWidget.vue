<script setup lang="ts">
import { ref } from 'vue';

import { DsCallout } from '../ds';
import { askChat, type ChatResponse } from '../../services/chatApi';
import ChatInput from './ChatInput.vue';
import ChatMessage from './ChatMessage.vue';

const HONEST_ERROR_MESSAGE =
  'Non riesco a rispondere ora. Riprova tra qualche minuto.';

interface Exchange {
  question: string;
  response: ChatResponse | null;
  failed: boolean;
}

const exchanges = ref<Exchange[]>([]);
const busy = ref(false);

async function ask(question: string): Promise<void> {
  const exchange: Exchange = { question, response: null, failed: false };
  exchanges.value.push(exchange);
  busy.value = true;

  try {
    exchange.response = await askChat(question);
  } catch {
    exchange.failed = true;
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <section class="chat-widget">
    <p v-if="exchanges.length === 0" class="chat-widget__empty">
      Fai una domanda su un servizio del Comune per iniziare.
    </p>

    <template v-for="(exchange, index) in exchanges" :key="index">
      <ChatMessage
        v-if="exchange.response"
        :question="exchange.question"
        :response="exchange.response"
      />
      <article v-else-if="exchange.failed" class="chat-message">
        <p class="chat-message__question">
          <strong>Tu:</strong> {{ exchange.question }}
        </p>
        <DsCallout variant="danger" title="Errore">{{
          HONEST_ERROR_MESSAGE
        }}</DsCallout>
      </article>
      <article v-else class="chat-message chat-message--pending">
        <p class="chat-message__question">
          <strong>Tu:</strong> {{ exchange.question }}
        </p>
        <p class="chat-message__pending" role="status">Sto rispondendo…</p>
      </article>
    </template>

    <ChatInput :busy="busy" @ask="ask" />
  </section>
</template>
