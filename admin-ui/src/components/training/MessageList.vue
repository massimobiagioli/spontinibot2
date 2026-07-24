<script setup lang="ts">
import { DsCallout } from '../ds';
import SpanFeedback from './SpanFeedback.vue';
import type { TrainingMessageResponse } from '../../services/adminApi';

defineProps<{
  messages: TrainingMessageResponse[];
}>();
</script>

<template>
  <section>
    <h2>Cronologia</h2>

    <article v-for="message in messages" :key="message.id">
      <p><strong>Domanda:</strong> {{ message.question }}</p>
      <SpanFeedback :message-id="message.id" :answer="message.answer" />

      <DsCallout
        v-if="message.fell_back"
        variant="warning"
        title="Nessuna informazione trovata"
      >
        Spontini non ha trovato informazioni nei documenti comunali per
        rispondere a questa domanda.
      </DsCallout>

      <details v-else>
        <summary>Fonti ({{ message.sources.length }})</summary>
        <ul>
          <li v-for="source in message.sources" :key="source.document_id">
            {{ source.source_ref }}
          </li>
        </ul>
      </details>
    </article>
  </section>
</template>
