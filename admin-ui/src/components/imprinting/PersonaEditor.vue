<script setup lang="ts">
import { ref, useId, watch } from 'vue';

import { DsButton, DsCallout, DsInput } from '../ds';
import {
  AdminApiError,
  createPersona,
  type PersonaResponse,
} from '../../services/adminApi';

const props = defineProps<{
  personaName: string;
  activeVersion: PersonaResponse | null;
}>();

const emit = defineEmits<{ saved: [persona: PersonaResponse] }>();

const name = ref(props.activeVersion?.name ?? props.personaName);
const systemPrompt = ref(props.activeVersion?.system_prompt ?? '');
const tone = ref(props.activeVersion?.tone ?? '');
const fallbackMessage = ref(props.activeVersion?.fallback_message ?? '');
const activate = ref(true);
const saving = ref(false);
const error = ref<string | null>(null);
const success = ref(false);

watch(
  () => props.activeVersion,
  (version) => {
    name.value = version?.name ?? props.personaName;
    systemPrompt.value = version?.system_prompt ?? '';
    tone.value = version?.tone ?? '';
    fallbackMessage.value = version?.fallback_message ?? '';
  },
);

const systemPromptId = useId();
const fallbackMessageId = useId();
const activateCheckboxId = useId();

async function save(): Promise<void> {
  saving.value = true;
  error.value = null;
  success.value = false;
  try {
    const persona = await createPersona({
      name: name.value,
      system_prompt: systemPrompt.value,
      ...(tone.value ? { tone: tone.value } : {}),
      ...(fallbackMessage.value
        ? { fallback_message: fallbackMessage.value }
        : {}),
      activate: activate.value,
    });
    success.value = true;
    emit('saved', persona);
  } catch (e) {
    error.value =
      e instanceof AdminApiError
        ? e.message
        : 'Impossibile salvare la persona. Riprova più tardi.';
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <form @submit.prevent="save">
    <h2>Persona</h2>

    <DsInput v-model="name" label="Nome persona" required />

    <div class="form-group">
      <label :for="systemPromptId">Prompt di sistema</label>
      <textarea
        :id="systemPromptId"
        v-model="systemPrompt"
        class="form-control"
        rows="6"
        required
      />
    </div>

    <DsInput v-model="tone" label="Tono" />

    <div class="form-group">
      <label :for="fallbackMessageId">Messaggio di fallback</label>
      <textarea
        :id="fallbackMessageId"
        v-model="fallbackMessage"
        class="form-control"
        rows="3"
      />
    </div>

    <div class="form-check">
      <input
        :id="activateCheckboxId"
        v-model="activate"
        type="checkbox"
        class="form-check-input"
      />
      <label :for="activateCheckboxId">Attiva questa versione</label>
    </div>

    <DsCallout v-if="success" variant="success" title="Versione salvata">
      La nuova versione della persona è stata salvata.
    </DsCallout>
    <p v-if="error" role="alert">{{ error }}</p>

    <DsButton type="submit" :disabled="saving">
      {{ saving ? 'Salvataggio…' : 'Salva nuova versione' }}
    </DsButton>
  </form>
</template>
