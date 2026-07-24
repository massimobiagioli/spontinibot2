<script setup lang="ts">
import { ref, watch } from 'vue';

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

// Visibility itself is driven by the `:open` binding below, which works
// everywhere (including jsdom, which has no HTMLDialogElement.showModal).
// showModal()/close() are called only as a progressive enhancement for real
// browsers, giving native modal semantics: focus trap, ::backdrop, Esc-to-cancel.
watch(
  () => props.open,
  (isOpen) => {
    const dialog = dialogRef.value;
    if (!dialog) return;
    if (isOpen && typeof dialog.showModal === 'function') dialog.showModal();
    else if (!isOpen && typeof dialog.close === 'function') dialog.close();
  },
  { immediate: true, flush: 'post' },
);

function confirm(): void {
  emit('confirm');
}

function cancel(): void {
  emit('cancel');
}
</script>

<template>
  <dialog ref="dialogRef" class="modal" :open="open" @cancel.prevent="cancel">
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
