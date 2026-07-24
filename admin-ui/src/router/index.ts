import { createRouter, createWebHistory } from 'vue-router';
import type { RouteRecordRaw } from 'vue-router';

import DevCatalog from '../views/DevCatalog.vue';
import HomeView from '../views/HomeView.vue';
import IngestView from '../views/IngestView.vue';

const routes: RouteRecordRaw[] = [
  { path: '/', name: 'home', component: HomeView },
  { path: '/dev', name: 'dev-catalog', component: DevCatalog },
  { path: '/ingest', name: 'ingest', component: IngestView },
];

export const router = createRouter({
  history: createWebHistory(),
  routes,
});
