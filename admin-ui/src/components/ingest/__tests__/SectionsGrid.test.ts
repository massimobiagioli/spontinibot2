import { mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
import { createRouter, createWebHistory } from 'vue-router';

import * as adminApi from '../../../services/adminApi';
import SectionsGrid from '../SectionsGrid.vue';

function section(overrides: Partial<adminApi.IngestSectionWithSources> = {}) {
  return {
    id: 1,
    name: 'news',
    ordering: 10,
    created_at: '2026-07-24T00:00:00Z',
    sources: [],
    curation_sources: [],
    ...overrides,
  };
}

function makeRouter() {
  return createRouter({
    history: createWebHistory(),
    routes: [
      { path: '/ingest', name: 'ingest', component: { template: '<div />' } },
      {
        path: '/ingest/sections/:id',
        name: 'ingest-section',
        component: { template: '<div />' },
      },
    ],
  });
}

describe('SectionsGrid', () => {
  it('renders each section as a card in a grid, with a link to its detail page', async () => {
    const router = makeRouter();
    await router.push('/ingest');
    await router.isReady();

    const wrapper = mount(SectionsGrid, {
      props: {
        sections: [
          section({ id: 1, name: 'news' }),
          section({ id: 2, name: 'sport' }),
        ],
      },
      global: { plugins: [router] },
    });

    expect(wrapper.text()).toContain('news');
    expect(wrapper.text()).toContain('sport');
    expect(wrapper.find('.row').exists()).toBe(true);
    expect(wrapper.findAllComponents({ name: 'RouterLink' })).toHaveLength(2);
    const link = wrapper.find('a[href="/ingest/sections/1"]');
    expect(link.exists()).toBe(true);
  });

  it('never shows the internal ordering value', () => {
    const wrapper = mount(SectionsGrid, {
      props: { sections: [section({ ordering: 999 })] },
      global: { plugins: [makeRouter()] },
    });

    expect(wrapper.text()).not.toContain('999');
    expect(wrapper.text().toLowerCase()).not.toContain('ordine');
  });

  it('shows how many sources are configured per section', () => {
    const wrapper = mount(SectionsGrid, {
      props: {
        sections: [
          section({
            id: 1,
            sources: [
              {
                id: 1,
                section_id: 1,
                source_type: 'scrape',
                url: 'https://example.com',
                enabled: true,
                created_at: 'now',
                coming_soon: false,
              },
            ],
          }),
        ],
      },
      global: { plugins: [makeRouter()] },
    });

    expect(wrapper.text()).toContain('1 fonte');
  });

  it('adding a section calls createSection with an auto-computed ordering, not asked from the user', async () => {
    const createSpy = vi
      .spyOn(adminApi, 'createSection')
      .mockResolvedValue(section({ id: 3, name: 'delibere', ordering: 30 }));

    const wrapper = mount(SectionsGrid, {
      props: {
        sections: [
          section({ id: 1, ordering: 20 }),
          section({ id: 2, ordering: 10 }),
        ],
      },
      global: { plugins: [makeRouter()] },
    });

    // no ordering input exists anywhere in the add form
    expect(wrapper.find('input[type="number"]').exists()).toBe(false);

    await wrapper.find('button').trigger('click'); // reveal add form
    await wrapper.find('input[type="text"]').setValue('delibere');
    await wrapper.find('form').trigger('submit');
    await Promise.resolve();
    await wrapper.vm.$nextTick();

    expect(createSpy).toHaveBeenCalledWith('delibere', 30); // max(20,10) + 10
    expect(wrapper.emitted('changed')).toHaveLength(1);
  });

  it('shows an honest error message when adding a section fails', async () => {
    vi.spyOn(adminApi, 'createSection').mockRejectedValue(
      new adminApi.AdminApiError(500, 'database error: connection refused'),
    );

    const wrapper = mount(SectionsGrid, {
      props: { sections: [] },
      global: { plugins: [makeRouter()] },
    });

    await wrapper.find('button').trigger('click');
    await wrapper.find('input[type="text"]').setValue('delibere');
    await wrapper.find('form').trigger('submit');
    await Promise.resolve();
    await wrapper.vm.$nextTick();

    expect(wrapper.text()).toContain('database error: connection refused');
    expect(wrapper.emitted('changed')).toBeUndefined();
  });

  it('shows an honest empty state when there are no sections yet', () => {
    const wrapper = mount(SectionsGrid, {
      props: { sections: [] },
      global: { plugins: [makeRouter()] },
    });

    expect(wrapper.text().toLowerCase()).toContain('nessuna sezione');
  });
});
