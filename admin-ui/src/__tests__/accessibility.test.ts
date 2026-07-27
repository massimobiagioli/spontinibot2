import axe from 'axe-core';
import { flushPromises, mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
import { createMemoryHistory, createRouter } from 'vue-router';

import App from '../App.vue';
import * as adminApi from '../services/adminApi';
import DevCatalog from '../views/DevCatalog.vue';
import HomeView from '../views/HomeView.vue';
import ImprintingView from '../views/ImprintingView.vue';
import IngestView from '../views/IngestView.vue';
import LoginView from '../views/LoginView.vue';
import TrainingSessionView from '../views/TrainingSessionView.vue';
import TrainingSessionsView from '../views/TrainingSessionsView.vue';

function makeRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/', component: HomeView },
      { path: '/login', name: 'login', component: LoginView },
      { path: '/dev', component: DevCatalog },
      { path: '/ingest', component: IngestView },
      { path: '/imprinting', component: ImprintingView },
      { path: '/training', component: TrainingSessionsView },
      { path: '/training/:id', component: TrainingSessionView },
    ],
  });
}

// jsdom cannot compute real layout/contrast, so this is a fast first-pass
// signal; pa11y (Task 4.2, real headless Chromium) is the authoritative gate.
async function runAxe(el: Element) {
  return axe.run(el, {
    rules: { 'color-contrast': { enabled: false } },
  });
}

describe('accessibility', () => {
  it('the app shell has zero axe violations', async () => {
    const router = makeRouter();
    await router.push('/');
    await router.isReady();

    const wrapper = mount(App, {
      global: { plugins: [router] },
      attachTo: document.body,
    });

    const results = await runAxe(wrapper.element);

    expect(results.violations).toEqual([]);

    wrapper.unmount();
  });

  it('the /dev catalog has zero axe violations', async () => {
    const router = makeRouter();
    await router.push('/dev');
    await router.isReady();

    const wrapper = mount(App, {
      global: { plugins: [router] },
      attachTo: document.body,
    });

    const results = await runAxe(wrapper.element);

    expect(results.violations).toEqual([]);

    wrapper.unmount();
  });

  it('the /login route has zero axe violations', async () => {
    const router = makeRouter();
    await router.push('/login');
    await router.isReady();

    const wrapper = mount(App, {
      global: { plugins: [router] },
      attachTo: document.body,
    });

    const results = await runAxe(wrapper.element);

    expect(results.violations).toEqual([]);

    wrapper.unmount();
  });

  it('the /ingest section has zero axe violations', async () => {
    vi.spyOn(adminApi, 'getIngestConfig').mockResolvedValue({
      schedule: {
        cron_expr: '0 */4 * * *',
        enabled: true,
        updated_at: '2026-07-24T00:00:00Z',
      },
      sections: [
        {
          id: 1,
          name: 'news',
          ordering: 10,
          created_at: '2026-07-24T00:00:00Z',
          sources: [
            {
              id: 1,
              section_id: 1,
              source_type: 'scrape',
              url: 'https://example.com/news',
              enabled: true,
              created_at: '2026-07-24T00:00:00Z',
              coming_soon: false,
            },
            {
              id: 2,
              section_id: 1,
              source_type: 'api',
              url: 'https://api.example.com',
              enabled: false,
              created_at: '2026-07-24T00:00:00Z',
              coming_soon: true,
            },
          ],
          curation_sources: [],
        },
      ],
    });

    const router = makeRouter();
    await router.push('/ingest');
    await router.isReady();

    const wrapper = mount(App, {
      global: { plugins: [router] },
      attachTo: document.body,
    });
    await flushPromises();

    const results = await runAxe(wrapper.element);

    expect(results.violations).toEqual([]);

    wrapper.unmount();
  });

  it('the /imprinting section has zero axe violations', async () => {
    vi.spyOn(adminApi, 'getPersonaVersions').mockResolvedValue([
      {
        id: 1,
        version: 1,
        name: 'gaspare',
        system_prompt: 'Sei Gaspare, compositore di Maiolati.',
        tone: 'solenne',
        fallback_message: 'Non ho trovato informazioni.',
        is_active: true,
        created_at: '2026-07-24T00:00:00Z',
        created_by: 'admin',
      },
    ]);

    const router = makeRouter();
    await router.push('/imprinting');
    await router.isReady();

    const wrapper = mount(App, {
      global: { plugins: [router] },
      attachTo: document.body,
    });
    await flushPromises();

    const results = await runAxe(wrapper.element);

    expect(results.violations).toEqual([]);

    wrapper.unmount();
  });

  it('the /training section has zero axe violations', async () => {
    vi.spyOn(adminApi, 'listSessions').mockResolvedValue([
      {
        id: 1,
        title: "Sessione sull'anagrafe",
        created_at: '2026-07-24T00:00:00Z',
        created_by: 'operator1',
        closed_at: null,
        notes: null,
      },
      {
        id: 2,
        title: 'Sessione chiusa',
        created_at: '2026-07-20T00:00:00Z',
        created_by: 'operator1',
        closed_at: '2026-07-21T00:00:00Z',
        notes: 'tutto ok',
      },
    ]);

    const router = makeRouter();
    await router.push('/training');
    await router.isReady();

    const wrapper = mount(App, {
      global: { plugins: [router] },
      attachTo: document.body,
    });
    await flushPromises();

    const results = await runAxe(wrapper.element);

    expect(results.violations).toEqual([]);

    wrapper.unmount();
  });

  it('the /training/:id session detail has zero axe violations', async () => {
    vi.spyOn(adminApi, 'getSession').mockResolvedValue({
      id: 1,
      title: "Sessione sull'anagrafe",
      created_at: '2026-07-24T00:00:00Z',
      created_by: 'operator1',
      closed_at: null,
      notes: null,
    });
    vi.spyOn(adminApi, 'listTrainingMessages').mockResolvedValue([
      {
        id: 1,
        session_id: 1,
        question: "A che ora apre l'anagrafe?",
        answer: 'Lo sportello apre alle 9:00. Il sabato è chiuso.',
        sources: [{ document_id: 7, source_ref: 'orari.md' }],
        fell_back: false,
        created_at: '2026-07-24T00:00:00Z',
        expected_answer: null,
        execution_time_ms: 84,
        source: 'chat',
      },
      {
        id: 2,
        session_id: 1,
        question: 'Qual è la popolazione della luna?',
        answer: 'Non ho trovato informazioni nei documenti comunali.',
        sources: [],
        fell_back: true,
        created_at: '2026-07-24T00:00:00Z',
        expected_answer: null,
        execution_time_ms: 120,
        source: 'chat',
      },
    ]);
    vi.spyOn(adminApi, 'listTrainingFeedback').mockResolvedValue([
      {
        id: 1,
        message_id: 1,
        chunk_id: null,
        answer_span: 'Lo sportello apre alle 9:00.',
        sentiment: 'positive',
        comment: null,
        created_at: '2026-07-24T00:00:00Z',
      },
    ]);

    const router = makeRouter();
    await router.push('/training/1');
    await router.isReady();

    const wrapper = mount(App, {
      global: { plugins: [router] },
      attachTo: document.body,
    });
    await flushPromises();

    const results = await runAxe(wrapper.element);

    expect(results.violations).toEqual([]);

    wrapper.unmount();
  });
});
