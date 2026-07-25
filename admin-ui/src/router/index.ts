import { createRouter, createWebHistory } from 'vue-router';
import type { RouteRecordRaw } from 'vue-router';

import { onUnauthorized } from '../services/adminApi';
import DevCatalog from '../views/DevCatalog.vue';
import HomeView from '../views/HomeView.vue';
import ImprintingView from '../views/ImprintingView.vue';
import IngestView from '../views/IngestView.vue';
import LoginView from '../views/LoginView.vue';
import TrainingSessionView from '../views/TrainingSessionView.vue';
import TrainingSessionsView from '../views/TrainingSessionsView.vue';

const routes: RouteRecordRaw[] = [
  { path: '/', name: 'home', component: HomeView },
  { path: '/login', name: 'login', component: LoginView },
  { path: '/dev', name: 'dev-catalog', component: DevCatalog },
  { path: '/ingest', name: 'ingest', component: IngestView },
  { path: '/imprinting', name: 'imprinting', component: ImprintingView },
  {
    path: '/training',
    name: 'training',
    component: TrainingSessionsView,
  },
  {
    path: '/training/:id',
    name: 'training-session',
    component: TrainingSessionView,
  },
];

export const router = createRouter({
  history: createWebHistory(),
  routes,
});

onUnauthorized(() => {
  if (router.currentRoute.value.name !== 'login') {
    void router.push({ name: 'login' });
  }
});
