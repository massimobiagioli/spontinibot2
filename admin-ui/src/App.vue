<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { useRoute } from 'vue-router';

import { DsNav } from './components/ds';

const businessLinks = [
  { to: '/ingest', label: 'Ingest', icon: 'it-upload' },
  { to: '/imprinting', label: 'Imprinting', icon: 'it-pencil' },
  { to: '/training', label: 'Training', icon: 'it-chart-line' },
];

// Below this width the sidebar becomes an off-canvas drawer (mirrors
// Bootstrap's `lg` breakpoint, 992px) instead of a permanent column.
const DESKTOP_QUERY = '(min-width: 992px)';
const supportsMatchMedia =
  typeof window !== 'undefined' && typeof window.matchMedia === 'function';
// Environments without matchMedia (e.g. jsdom in tests) default to the
// always-visible desktop layout rather than a hidden/inert sidebar.
const isDesktop = ref(
  !supportsMatchMedia || window.matchMedia(DESKTOP_QUERY).matches,
);
const navOpen = ref(false);
// On desktop the sidebar is always present and interactive; on mobile it's
// only present/interactive while explicitly opened. Keeping both states
// distinct (rather than one boolean) lets the sidebar stay `inert` — and
// out of the tab order — while off-canvas and closed.
const navVisible = computed(() => isDesktop.value || navOpen.value);

let desktopQuery: MediaQueryList | undefined;
function handleDesktopChange(event: MediaQueryListEvent): void {
  isDesktop.value = event.matches;
  if (event.matches) navOpen.value = false;
}

onMounted(() => {
  if (!supportsMatchMedia) return;
  desktopQuery = window.matchMedia(DESKTOP_QUERY);
  desktopQuery.addEventListener('change', handleDesktopChange);
});

onUnmounted(() => {
  desktopQuery?.removeEventListener('change', handleDesktopChange);
});

function toggleNav(): void {
  navOpen.value = !navOpen.value;
}

function closeNav(): void {
  navOpen.value = false;
}

const route = useRoute();
watch(
  () => route.fullPath,
  () => {
    navOpen.value = false;
  },
);
</script>

<template>
  <div class="app-shell">
    <header class="app-shell__header">
      <button
        type="button"
        class="app-shell__nav-toggle"
        aria-controls="app-shell-nav"
        aria-label="Menu di navigazione"
        :aria-expanded="navVisible"
        @click="toggleNav"
      >
        <svg class="icon icon-inverse" aria-hidden="true">
          <use
            :href="`/sprites.svg#${navOpen ? 'it-close-big' : 'it-burger'}`"
          />
        </svg>
      </button>
      <span class="app-shell__brand">
        <span class="app-shell__brand-icon" aria-hidden="true">🎼</span>
        <span class="app-shell__brand-text"
          >Comune di Maiolati Spontini — Pannello di Controllo</span
        >
      </span>
    </header>
    <div class="app-shell__body">
      <div
        v-if="navOpen && !isDesktop"
        class="app-shell__nav-backdrop"
        @click="closeNav"
      ></div>
      <DsNav
        id="app-shell-nav"
        class="app-shell__nav"
        :class="{ 'app-shell__nav--open': navOpen }"
        :inert="!navVisible"
        :links="businessLinks"
      />
      <main class="app-shell__main">
        <RouterView />
      </main>
    </div>
    <footer class="app-shell__footer">
      Comune di Maiolati Spontini — Spontini Bot
    </footer>
  </div>
</template>
