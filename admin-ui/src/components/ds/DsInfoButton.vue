<script setup lang="ts">
import { ref } from 'vue';

withDefaults(
  defineProps<{
    label?: string;
    title: string;
  }>(),
  {
    label: 'Informazioni',
  },
);

const dialogRef = ref<HTMLDialogElement | null>(null);

// jsdom has no showModal()/close() (see HTMLDialogElement-impl.js), so tests
// fall back to toggling the `open` attribute directly — same guard pattern
// as DsConfirmDialog.
function open(): void {
  const dialog = dialogRef.value;
  if (!dialog) return;
  if (typeof dialog.showModal === 'function') dialog.showModal();
  else dialog.setAttribute('open', '');
}

function close(): void {
  const dialog = dialogRef.value;
  if (!dialog) return;
  if (typeof dialog.close === 'function') dialog.close();
  else dialog.removeAttribute('open');
}
</script>

<template>
  <button
    type="button"
    class="ds-info-button"
    :aria-label="label"
    :title="label"
    @click="open"
  >
    i
  </button>

  <dialog ref="dialogRef" class="ds-info-dialog" @cancel.prevent="close">
    <div class="ds-info-dialog__content">
      <header class="ds-info-dialog__header">
        <h2>{{ title }}</h2>
        <button
          type="button"
          class="ds-info-dialog__close-icon"
          aria-label="Chiudi"
          @click="close"
        >
          &times;
        </button>
      </header>

      <div class="ds-info-dialog__body">
        <slot />
      </div>

      <footer class="ds-info-dialog__footer">
        <button type="button" class="btn btn-outline-secondary" @click="close">
          Chiudi
        </button>
      </footer>
    </div>
  </dialog>
</template>

<style scoped>
.ds-info-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 1.25rem;
  height: 1.25rem;
  padding: 0;
  border: 1px solid var(--it-color-primary, #1565c0);
  border-radius: 50%;
  background: none;
  color: var(--it-color-primary, #1565c0);
  font-size: 0.75rem;
  font-style: italic;
  font-weight: 700;
  line-height: 1;
  cursor: pointer;
}

.ds-info-button:hover,
.ds-info-button:focus-visible {
  background: var(--it-color-primary, #1565c0);
  color: var(--it-color-white, #fff);
}

.ds-info-dialog {
  border: none;
  border-radius: 8px;
  padding: 0;
  width: min(32rem, 90vw);
  max-height: 85vh;
  overflow: hidden;
  box-shadow: var(--it-elevation-medium, 0 4px 16px rgba(0, 0, 0, 0.25));
}

.ds-info-dialog::backdrop {
  background: rgba(0, 0, 0, 0.5);
}

.ds-info-dialog__content {
  display: flex;
  flex-direction: column;
  max-height: 85vh;
}

.ds-info-dialog__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 1.25rem 1.5rem;
  border-bottom: 1px solid rgba(31, 42, 55, 0.08);
}

.ds-info-dialog__header h2 {
  margin: 0;
  font-size: 1.25rem;
}

.ds-info-dialog__close-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 44px;
  min-width: 44px;
  border: none;
  border-radius: 4px;
  background: none;
  font-size: 1.5rem;
  line-height: 1;
  color: var(--spontini-color-text-muted, #6c757d);
  cursor: pointer;
}

.ds-info-dialog__close-icon:hover,
.ds-info-dialog__close-icon:focus-visible {
  background: rgba(31, 42, 55, 0.06);
  color: var(--spontini-color-text, #1f2a37);
}

.ds-info-dialog__body {
  padding: 1.5rem;
  overflow-y: auto;
  line-height: 1.5;
}

.ds-info-dialog__footer {
  display: flex;
  justify-content: flex-end;
  padding: 1rem 1.5rem;
  border-top: 1px solid rgba(31, 42, 55, 0.08);
}
</style>
