<script setup lang="ts">
import { ref } from 'vue';

import { DsCallout } from '../ds';
import {
  askChat,
  ChatApiError,
  type ChatResponse,
} from '../../services/chatApi';
import ChatInput from './ChatInput.vue';
import ChatMessage from './ChatMessage.vue';

interface Exchange {
  question: string;
  response: ChatResponse | null;
  error: string | null;
}

const exchanges = ref<Exchange[]>([]);
const busy = ref(false);

async function ask(question: string): Promise<void> {
  const exchange: Exchange = { question, response: null, error: null };
  exchanges.value.push(exchange);
  busy.value = true;

  try {
    exchange.response = await askChat(question);
  } catch (e) {
    exchange.error =
      e instanceof ChatApiError
        ? e.message
        : 'Non riesco a rispondere ora. Riprova più tardi.';
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
      <article v-else-if="exchange.error" class="chat-message">
        <p class="chat-message__question">
          <strong>Tu:</strong> {{ exchange.question }}
        </p>
        <DsCallout variant="danger" title="Errore">{{
          exchange.error
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
