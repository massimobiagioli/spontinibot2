import { flushPromises, mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
import { createRouter, createWebHistory } from 'vue-router';

import * as adminApi from '../../services/adminApi';
import IngestView from '../IngestView.vue';

function makeRouter() {
  return createRouter({
    history: createWebHistory(),
    routes: [
      { path: '/ingest', name: 'ingest', component: IngestView },
      {
        path: '/ingest/sections/:id',
        name: 'ingest-section',
        component: { template: '<div />' },
      },
    ],
  });
}

describe('IngestView', () => {
  it('renders a loading state and then the resolved configuration', async () => {
    const getIngestConfigSpy = vi
      .spyOn(adminApi, 'getIngestConfig')
      .mockResolvedValue({
        schedule: null,
        sections: [
          { id: 1, name: 'news', ordering: 10, created_at: 'now', sources: [] },
        ],
      });

    const wrapper = mount(IngestView, { global: { plugins: [makeRouter()] } });

    expect(wrapper.text()).toContain('Caricamento della configurazione');

    await flushPromises();

    expect(getIngestConfigSpy).toHaveBeenCalledOnce();
    expect(wrapper.text()).not.toContain('Caricamento della configurazione');
    expect(wrapper.text()).toContain('news');
  });

  it('renders an honest error state when the config fetch fails', async () => {
    vi.spyOn(adminApi, 'getIngestConfig').mockRejectedValue(
      new adminApi.AdminApiError(401, 'invalid or missing session cookie'),
    );

    const wrapper = mount(IngestView, { global: { plugins: [makeRouter()] } });
    await flushPromises();

    expect(wrapper.text()).toContain('invalid or missing session cookie');
  });
});
