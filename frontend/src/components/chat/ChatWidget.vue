<script setup lang="ts">
import { nextTick, ref, useTemplateRef, watch } from 'vue';

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
const isOpen = ref(false);
const panel = useTemplateRef<HTMLElement>('panel');

function toggle(): void {
  isOpen.value = !isOpen.value;
}

function close(): void {
  isOpen.value = false;
}

watch(isOpen, (open) => {
  if (!open) return;
  nextTick(() => panel.value?.focus());
});

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
  <div class="chat-widget">
    <button
      type="button"
      class="chat-widget__toggle"
      :aria-expanded="isOpen"
      aria-controls="chat-widget-panel"
      @click="toggle"
    >
      <span aria-hidden="true">{{ isOpen ? '✕' : '🎼' }}</span>
      <span class="visually-hidden">{{
        isOpen
          ? 'Chiudi la chat con Spontini Bot'
          : 'Apri la chat con Spontini Bot'
      }}</span>
    </button>

    <section
      v-if="isOpen"
      id="chat-widget-panel"
      ref="panel"
      class="chat-widget__panel"
      role="dialog"
      aria-label="Chat con Spontini Bot"
      tabindex="-1"
      @keydown.esc="close"
    >
      <header class="chat-widget__header">
        <span class="chat-widget__avatar" aria-hidden="true">🎼</span>
        <div class="chat-widget__header-text">
          <p class="chat-widget__title">
            Spontini Bot
            <span class="chat-widget__status">
              <span class="chat-widget__status-dot" aria-hidden="true"></span>
              Online
            </span>
          </p>
          <p class="chat-widget__subtitle">Comune di Maiolati Spontini</p>
        </div>
        <button
          type="button"
          class="chat-widget__close"
          aria-label="Chiudi la chat"
          @click="close"
        >
          ✕
        </button>
      </header>

      <div class="chat-widget__messages">
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
            <div class="chat-message__row chat-message__row--user">
              <p class="chat-message__question chat-message__bubble">
                <strong>Tu:</strong> {{ exchange.question }}
              </p>
            </div>
            <div class="chat-message__row chat-message__row--bot">
              <span class="chat-message__avatar" aria-hidden="true">🎼</span>
              <div class="chat-message__bubble">
                <DsCallout variant="danger" title="Errore">{{
                  HONEST_ERROR_MESSAGE
                }}</DsCallout>
              </div>
            </div>
          </article>
          <article v-else class="chat-message chat-message--pending">
            <div class="chat-message__row chat-message__row--user">
              <p class="chat-message__question chat-message__bubble">
                <strong>Tu:</strong> {{ exchange.question }}
              </p>
            </div>
            <div class="chat-message__row chat-message__row--bot">
              <span class="chat-message__avatar" aria-hidden="true">🎼</span>
              <p class="chat-widget__typing" role="status">
                <span class="chat-widget__typing-dot"></span>
                <span class="chat-widget__typing-dot"></span>
                <span class="chat-widget__typing-dot"></span>
                <span class="visually-hidden">Sto rispondendo…</span>
              </p>
            </div>
          </article>
        </template>
      </div>

      <ChatInput :busy="busy" @ask="ask" />
    </section>
  </div>
</template>
