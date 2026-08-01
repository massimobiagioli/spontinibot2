<script setup lang="ts">
import { ref, useId, watch } from 'vue';

import { DsButton, DsInput } from '../ds';
import {
  AdminApiError,
  upsertSchedule,
  type IngestScheduleResponse,
} from '../../services/adminApi';

const props = defineProps<{
  schedule: IngestScheduleResponse | null;
}>();

const emit = defineEmits<{ saved: [schedule: IngestScheduleResponse] }>();

const cronExpr = ref(props.schedule?.cron_expr ?? '');
const enabled = ref(props.schedule?.enabled ?? false);
const saving = ref(false);
const error = ref<string | null>(null);

watch(
  () => props.schedule,
  (schedule) => {
    cronExpr.value = schedule?.cron_expr ?? '';
    enabled.value = schedule?.enabled ?? false;
  },
);

const enabledCheckboxId = useId();

async function save(): Promise<void> {
  saving.value = true;
  error.value = null;
  try {
    const result = await upsertSchedule(cronExpr.value, enabled.value);
    emit('saved', result);
  } catch (e) {
    error.value =
      e instanceof AdminApiError
        ? e.message
        : 'Impossibile salvare la pianificazione. Riprova più tardi.';
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <form @submit.prevent="save">
    <DsInput
      v-model="cronExpr"
      label="Espressione cron"
      hint='Formato standard a 5 campi (Minuto Ora GiornoMese Mese GiornoSettimana), es. "0 */4 * * *" per ogni 4 ore.'
      required
    />

    <div class="form-check">
      <input
        :id="enabledCheckboxId"
        v-model="enabled"
        type="checkbox"
        class="form-check-input"
      />
      <label :for="enabledCheckboxId">Pianificazione attiva</label>
    </div>

    <p v-if="error" role="alert">{{ error }}</p>

    <DsButton type="submit" :disabled="saving">
      {{ saving ? 'Salvataggio…' : 'Salva' }}
    </DsButton>
  </form>
</template>
