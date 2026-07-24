import axe from 'axe-core';
import { flushPromises, mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
import { createMemoryHistory, createRouter } from 'vue-router';

import App from '../App.vue';
import * as adminApi from '../services/adminApi';
import DevCatalog from '../views/DevCatalog.vue';
import HomeView from '../views/HomeView.vue';
import IngestView from '../views/IngestView.vue';

function makeRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/', component: HomeView },
      { path: '/dev', component: DevCatalog },
      { path: '/ingest', component: IngestView },
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
});
