import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createRouter, createWebHistory } from 'vue-router';

import * as adminApi from '../../services/adminApi';
import SectionDetailView from '../SectionDetailView.vue';

function config(
  overrides: Partial<adminApi.IngestConfigResponse> = {},
): adminApi.IngestConfigResponse {
  return {
    schedule: null,
    sections: [
      {
        id: 1,
        name: 'news',
        ordering: 10,
        created_at: '2026-07-24T00:00:00Z',
        sources: [],
      },
    ],
    ...overrides,
  };
}

function makeRouter() {
  const router = createRouter({
    history: createWebHistory(),
    routes: [
      { path: '/ingest', name: 'ingest', component: { template: '<div />' } },
      {
        path: '/ingest/sections/:id',
        name: 'ingest-section',
        component: SectionDetailView,
      },
    ],
  });
  return router;
}

describe('SectionDetailView', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it('loads the config and shows the matching section by route id', async () => {
    vi.spyOn(adminApi, 'getIngestConfig').mockResolvedValue(config());
    const router = makeRouter();
    await router.push('/ingest/sections/1');
    await router.isReady();

    const wrapper = mount(SectionDetailView, {
      global: { plugins: [router] },
    });
    await flushPromises();

    expect(wrapper.find('h1').text()).toBe('news');
  });

  it('shows an honest not-found state when the section id does not exist', async () => {
    vi.spyOn(adminApi, 'getIngestConfig').mockResolvedValue(config());
    const router = makeRouter();
    await router.push('/ingest/sections/999');
    await router.isReady();

    const wrapper = mount(SectionDetailView, {
      global: { plugins: [router] },
    });
    await flushPromises();

    expect(wrapper.text().toLowerCase()).toContain('non trovata');
  });

  it('deleting the section requires confirmation, then navigates back to the sections list', async () => {
    vi.spyOn(adminApi, 'getIngestConfig').mockResolvedValue(config());
    const deleteSpy = vi
      .spyOn(adminApi, 'deleteSection')
      .mockResolvedValue(true);
    const router = makeRouter();
    await router.push('/ingest/sections/1');
    await router.isReady();
    const pushSpy = vi.spyOn(router, 'push');

    const wrapper = mount(SectionDetailView, {
      global: { plugins: [router] },
    });
    await flushPromises();

    await wrapper
      .find('[data-testid="delete-section-button"]')
      .trigger('click');
    await wrapper.vm.$nextTick();

    expect(deleteSpy).not.toHaveBeenCalled();
    const dialog = wrapper.find<HTMLDialogElement>(
      '[data-testid="section-delete-dialog"]',
    );
    expect(dialog.element.open).toBe(true);

    await dialog.find('button.btn-danger').trigger('click');
    await flushPromises();

    expect(deleteSpy).toHaveBeenCalledWith(1);
    expect(pushSpy).toHaveBeenCalledWith({ name: 'ingest' });
  });
});
