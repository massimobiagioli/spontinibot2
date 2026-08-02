<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useRouter } from 'vue-router';

import { DsButton, DsConfirmDialog, DsInput, DsPagination } from '../ds';
import {
  AdminApiError,
  createSession,
  deleteSession,
  type TrainingSessionResponse,
} from '../../services/adminApi';

const router = useRouter();

const props = defineProps<{
  sessions: TrainingSessionResponse[];
}>();

const emit = defineEmits<{ changed: [] }>();

// Block-style pagination — 9 cards per page (3x3 on desktop) keeps each
// page short enough to scan without an unbounded scrolling list.
const PAGE_SIZE = 9;
const currentPage = ref(1);
const totalPages = computed(() =>
  Math.max(1, Math.ceil(props.sessions.length / PAGE_SIZE)),
);
const pagedSessions = computed(() => {
  const start = (currentPage.value - 1) * PAGE_SIZE;
  return props.sessions.slice(start, start + PAGE_SIZE);
});

// Deleting the last session on a page (or the list shrinking otherwise)
// can leave currentPage pointing past the new last page — clamp it back.
watch(totalPages, (pages) => {
  if (currentPage.value > pages) currentPage.value = pages;
});

const newSessionTitle = ref('');
const addError = ref<string | null>(null);
const adding = ref(false);

async function addSession(): Promise<void> {
  adding.value = true;
  addError.value = null;
  try {
    await createSession(newSessionTitle.value);
    newSessionTitle.value = '';
    emit('changed');
  } catch (e) {
    addError.value =
      e instanceof AdminApiError
        ? e.message
        : 'Impossibile creare la sessione. Riprova più tardi.';
  } finally {
    adding.value = false;
  }
}

function goToSession(id: number): void {
  router.push(`/training/${id}`);
}

const pendingDeleteId = ref<number | null>(null);
const deleteError = ref<string | null>(null);

function requestDelete(id: number): void {
  deleteError.value = null;
  pendingDeleteId.value = id;
}

function cancelDelete(): void {
  pendingDeleteId.value = null;
}

async function confirmDelete(): Promise<void> {
  const id = pendingDeleteId.value;
  if (id === null) return;
  try {
    await deleteSession(id);
    pendingDeleteId.value = null;
    emit('changed');
  } catch (e) {
    deleteError.value =
      e instanceof AdminApiError
        ? e.message
        : 'Impossibile eliminare la sessione. Riprova più tardi.';
    pendingDeleteId.value = null;
  }
}
</script>

<template>
  <section>
    <h2>Sessioni</h2>

    <p v-if="deleteError" role="alert">{{ deleteError }}</p>

    <ul class="row list-unstyled session-list__grid">
      <li
        v-for="session in pagedSessions"
        :key="session.id"
        class="col-12 col-sm-6 col-lg-4 session-list__col"
      >
        <div
          class="it-card clickable-card session-list__card"
          @click="goToSession(session.id)"
        >
          <div class="session-list__card-header">
            <RouterLink
              :to="`/training/${session.id}`"
              class="it-card-title session-list__title"
              @click.stop
            >
              {{ session.title }}
            </RouterLink>
            <span v-if="session.closed_at" class="badge badge-secondary">
              Chiusa
            </span>
            <span v-else class="badge badge-success">Aperta</span>
          </div>
          <p class="session-list__date">{{ session.created_at }}</p>
          <div class="it-card-footer">
            <DsButton
              variant="danger"
              outline
              @click.stop="requestDelete(session.id)"
            >
              Elimina sessione
            </DsButton>
          </div>
        </div>
      </li>
    </ul>

    <DsPagination
      v-model:current-page="currentPage"
      :total-pages="totalPages"
      label="Paginazione sessioni"
    />

    <form @submit.prevent="addSession">
      <h3>Nuova sessione</h3>
      <DsInput v-model="newSessionTitle" label="Titolo sessione" required />
      <p v-if="addError" role="alert">{{ addError }}</p>
      <DsButton type="submit" :disabled="adding">
        {{ adding ? 'Creazione…' : 'Crea sessione' }}
      </DsButton>
    </form>

    <DsConfirmDialog
      data-testid="delete-session-dialog"
      :open="pendingDeleteId !== null"
      message="Eliminare questa sessione e tutte le sue domande? L'operazione non può essere annullata."
      confirm-label="Elimina"
      @confirm="confirmDelete"
      @cancel="cancelDelete"
    />
  </section>
</template>

<style scoped>
.session-list__grid {
  margin-top: 1rem;
}

.session-list__col {
  margin-bottom: 1.5rem;
}

.session-list__card {
  height: 100%;
}

/* DSI's own .it-card-title-icon (justify-content: space-between, no wrap)
   is built for a short icon+title pair, not a freeform title sharing a
   line with a status badge — a long session title left no room for the
   badge and crushed it against the text. Stack them instead: the title
   gets the full card width to wrap onto as many lines as it needs, and
   the badge always sits clearly below it. */
.session-list__card-header {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.5rem;
  margin-bottom: var(--bs-spacing-xxs, 0.5rem);
}

.session-list__title {
  display: block;
}

.session-list__date {
  color: var(--spontini-color-text-muted, #6c757d);
  font-size: 0.85rem;
}
</style>
