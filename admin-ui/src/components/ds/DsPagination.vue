<script setup lang="ts">
const props = defineProps<{
  currentPage: number;
  totalPages: number;
  /** Accessible name for the <nav> — must describe what is being paginated. */
  label: string;
}>();

const emit = defineEmits<{ 'update:currentPage': [page: number] }>();

function goToPage(page: number): void {
  if (page < 1 || page > props.totalPages || page === props.currentPage) return;
  emit('update:currentPage', page);
}
</script>

<template>
  <nav
    v-if="totalPages > 1"
    class="pagination-wrapper justify-content-center"
    :aria-label="label"
  >
    <ul class="pagination list-unstyled">
      <li class="page-item">
        <button
          type="button"
          class="page-link"
          :disabled="currentPage === 1"
          aria-label="Pagina precedente"
          @click="goToPage(currentPage - 1)"
        >
          <svg class="icon" aria-hidden="true">
            <use href="/sprites.svg#it-chevron-left" />
          </svg>
        </button>
      </li>
      <li v-for="page in totalPages" :key="page" class="page-item">
        <button
          type="button"
          class="page-link"
          :aria-current="page === currentPage ? 'page' : undefined"
          @click="goToPage(page)"
        >
          {{ page }}
        </button>
      </li>
      <li class="page-item">
        <button
          type="button"
          class="page-link"
          :disabled="currentPage === totalPages"
          aria-label="Pagina successiva"
          @click="goToPage(currentPage + 1)"
        >
          <svg class="icon" aria-hidden="true">
            <use href="/sprites.svg#it-chevron-right" />
          </svg>
        </button>
      </li>
    </ul>
  </nav>
</template>
