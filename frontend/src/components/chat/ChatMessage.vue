<script setup lang="ts">
import { DsCallout } from '../ds';
import type { ChatResponse } from '../../services/chatApi';

defineProps<{
  question: string;
  response: ChatResponse;
}>();

function isLink(sourceRef: string): boolean {
  return /^https?:\/\//.test(sourceRef);
}
</script>

<template>
  <article class="chat-message">
    <div class="chat-message__row chat-message__row--user">
      <p class="chat-message__question chat-message__bubble">
        <strong>Tu:</strong> {{ question }}
      </p>
    </div>

    <div class="chat-message__row chat-message__row--bot">
      <span class="chat-message__avatar" aria-hidden="true">🎼</span>
      <div class="chat-message__bubble">
        <DsCallout v-if="response.fell_back" variant="primary" role="status">
          {{ response.answer }}
        </DsCallout>
        <p v-else class="chat-message__answer">{{ response.answer }}</p>

        <details
          v-if="!response.fell_back && response.sources.length > 0"
          class="chat-message__sources"
        >
          <summary>Fonti ({{ response.sources.length }})</summary>
          <ul>
            <li v-for="source in response.sources" :key="source.document_id">
              <a
                v-if="isLink(source.source_ref)"
                :href="source.source_ref"
                target="_blank"
                rel="noopener noreferrer"
              >
                {{ source.source_ref }}
              </a>
              <span v-else>{{ source.source_ref }}</span>
            </li>
          </ul>
        </details>
      </div>
    </div>
  </article>
</template>
