<script setup lang="ts">
interface DsNavLink {
  to?: string;
  label: string;
  // Official Design Italia icon id (bootstrap-italia/dist/svg/sprites.svg),
  // e.g. "it-upload" — optional, purely decorative.
  icon?: string;
}

withDefaults(
  defineProps<{
    links: DsNavLink[];
    ariaLabel?: string;
  }>(),
  {
    ariaLabel: 'Navigazione principale',
  },
);
</script>

<template>
  <nav class="sidebar-wrapper" :aria-label="ariaLabel">
    <div class="sidebar-linklist-wrapper">
      <div class="link-list-wrapper">
        <ul class="link-list">
          <li v-for="link in links" :key="link.label">
            <RouterLink v-if="link.to" :to="link.to" class="list-item">
              <svg v-if="link.icon" class="icon icon-sm" aria-hidden="true">
                <use :href="`/sprites.svg#${link.icon}`" />
              </svg>
              <span>{{ link.label }}</span>
            </RouterLink>
            <span v-else class="list-item disabled" aria-disabled="true">
              <svg v-if="link.icon" class="icon icon-sm" aria-hidden="true">
                <use :href="`/sprites.svg#${link.icon}`" />
              </svg>
              <span>{{ link.label }}</span>
            </span>
          </li>
        </ul>
      </div>
    </div>
  </nav>
</template>
