import { createRouter, createWebHistory } from 'vue-router';
import type { RouteRecordRaw } from 'vue-router';

import DevCatalog from '../views/DevCatalog.vue';
import HomeView from '../views/HomeView.vue';

const routes: RouteRecordRaw[] = [
  { path: '/', name: 'home', component: HomeView },
  { path: '/dev', name: 'dev-catalog', component: DevCatalog },
];

export const router = createRouter({
  history: createWebHistory(),
  routes,
});
