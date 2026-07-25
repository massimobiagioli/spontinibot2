<script setup lang="ts">
import { DsCallout } from '../ds';
import type { ChatResponse } from '../../services/chatApi';

defineProps<{
  question: string;
  response: ChatResponse;
}>();
</script>

<template>
  <article class="chat-message">
    <p class="chat-message__question"><strong>Tu:</strong> {{ question }}</p>

    <DsCallout v-if="response.fell_back" variant="primary">
      {{ response.answer }}
    </DsCallout>
    <p v-else class="chat-message__answer">{{ response.answer }}</p>

    <details v-if="!response.fell_back && response.sources.length > 0">
      <summary>Fonti ({{ response.sources.length }})</summary>
      <ul>
        <li v-for="source in response.sources" :key="source.document_id">
          {{ source.source_ref }}
        </li>
      </ul>
    </details>
  </article>
</template>
