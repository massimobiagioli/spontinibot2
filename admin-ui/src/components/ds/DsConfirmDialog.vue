<script setup lang="ts">
import { onMounted, ref, watch } from 'vue';

const props = withDefaults(
  defineProps<{
    open: boolean;
    message: string;
    confirmLabel?: string;
    cancelLabel?: string;
  }>(),
  {
    confirmLabel: 'Conferma',
    cancelLabel: 'Annulla',
  },
);

const emit = defineEmits<{ confirm: []; cancel: [] }>();

const dialogRef = ref<HTMLDialogElement | null>(null);

// The `open` attribute is never bound in the template — it is driven
// entirely here, imperatively. In real browsers, showModal()/close() manage
// it natively, giving proper modal semantics: top-layer stacking, ::backdrop,
// focus trap, Esc-to-cancel. jsdom has no showModal()/close() (see
// HTMLDialogElement-impl.js), so there we fall back to toggling the `open`
// attribute directly, which is enough for tests to observe `.open`.
// Calling showModal() while `open` is already set throws an
// InvalidStateError, which is why the attribute is never template-bound.
function syncOpenState(isOpen: boolean): void {
  const dialog = dialogRef.value;
  if (!dialog) return;
  if (isOpen) {
    if (typeof dialog.showModal === 'function') dialog.showModal();
    else dialog.setAttribute('open', '');
  } else {
    if (typeof dialog.close === 'function') dialog.close();
    else dialog.removeAttribute('open');
  }
}

// Applied synchronously on mount (dialogRef is already bound at this point),
// rather than via the watcher's own `immediate` call, so that `.open`
// reflects the initial prop value the moment the component finishes mounting
// instead of one tick later.
onMounted(() => syncOpenState(props.open));

watch(() => props.open, syncOpenState);

function confirm(): void {
  emit('confirm');
}

function cancel(): void {
  emit('cancel');
}
</script>

<template>
  <dialog ref="dialogRef" class="ds-confirm-dialog" @cancel.prevent="cancel">
    <div class="modal-content">
      <div class="modal-body">
        <p>{{ message }}</p>
      </div>
      <div class="modal-footer">
        <button type="button" class="btn btn-outline-secondary" @click="cancel">
          {{ cancelLabel }}
        </button>
        <button type="button" class="btn btn-danger" autofocus @click="confirm">
          {{ confirmLabel }}
        </button>
      </div>
    </div>
  </dialog>
</template>

<style scoped>
.ds-confirm-dialog {
  border: 1px solid rgba(31, 42, 55, 0.12);
  border-radius: 8px;
  padding: 0;
  width: min(28rem, 90vw);
  /* This dialog doesn't use Bootstrap Italia's `.modal-content` class (only
     `.modal-body`/`.modal-footer` for their flex/spacing rules), so it never
     inherited BSI's `.modal-content` box-shadow — give it one directly,
     otherwise the panel has no visual separation from `::backdrop`. */
  box-shadow: var(--it-elevation-medium, 0 4px 16px rgba(0, 0, 0, 0.25));
}

.ds-confirm-dialog .modal-body {
  padding-top: 1.5rem;
}

.ds-confirm-dialog::backdrop {
  background: rgba(0, 0, 0, 0.5);
}
</style>
