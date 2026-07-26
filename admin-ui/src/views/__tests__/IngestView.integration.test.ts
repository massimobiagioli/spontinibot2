import { flushPromises, mount } from '@vue/test-utils';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createRouter, createWebHistory } from 'vue-router';
import { defineComponent, h } from 'vue';
import { RouterView } from 'vue-router';

import * as adminApi from '../../services/adminApi';
import IngestView from '../IngestView.vue';
import SectionDetailView from '../SectionDetailView.vue';

// The real app renders routed views inside <RouterView> (App.vue) — mounting
// IngestView directly would make in-app navigation (RouterLink clicks,
// router.push) invisible to the test, since nothing would swap the rendered
// component. This root mirrors that structure.
const RootWithRouterView = defineComponent({
  render: () => h(RouterView),
});

function makeRouter() {
  return createRouter({
    history: createWebHistory(),
    routes: [
      { path: '/ingest', name: 'ingest', component: IngestView },
      {
        path: '/ingest/sections/:id',
        name: 'ingest-section',
        component: SectionDetailView,
      },
    ],
  });
}

function findButtonByText(
  wrapper: { findAll: (s: string) => any[] },
  text: string,
) {
  const button = wrapper.findAll('button').find((b) => b.text() === text);
  if (!button) throw new Error(`button with text "${text}" not found`);
  return button;
}

describe('IngestView integration scenario', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('add a section, open its detail page, add a scraper source, trigger a run, see the run status', async () => {
    // Given an empty ingest configuration
    const getConfigSpy = vi
      .spyOn(adminApi, 'getIngestConfig')
      .mockResolvedValueOnce({ schedule: null, sections: [] })
      .mockResolvedValueOnce({
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
      })
      .mockResolvedValue({
        schedule: null,
        sections: [
          {
            id: 1,
            name: 'news',
            ordering: 10,
            created_at: '2026-07-24T00:00:00Z',
            sources: [
              {
                id: 100,
                section_id: 1,
                source_type: 'scrape',
                url: 'https://example.com/news',
                enabled: true,
                created_at: '2026-07-24T00:00:00Z',
                coming_soon: false,
              },
            ],
          },
        ],
      });

    vi.spyOn(adminApi, 'createSection').mockResolvedValue({
      id: 1,
      name: 'news',
      ordering: 10,
      created_at: '2026-07-24T00:00:00Z',
    });
    vi.spyOn(adminApi, 'createSource').mockResolvedValue({
      id: 100,
      section_id: 1,
      source_type: 'scrape',
      url: 'https://example.com/news',
      enabled: true,
      created_at: '2026-07-24T00:00:00Z',
      coming_soon: false,
    });
    vi.spyOn(adminApi, 'triggerIngestRun').mockResolvedValue({
      id: 1,
      status: 'pending',
      requested_at: '2026-07-24T00:00:00Z',
    });
    vi.spyOn(adminApi, 'getIngestRun').mockResolvedValue({
      id: 1,
      status: 'done',
      requested_at: '2026-07-24T00:00:00Z',
    });

    const router = makeRouter();
    await router.push('/ingest');
    await router.isReady();

    const wrapper = mount(RootWithRouterView, {
      global: { plugins: [router] },
    });
    await vi.advanceTimersByTimeAsync(0);
    expect(getConfigSpy).toHaveBeenCalledTimes(1);
    expect(wrapper.text()).not.toContain('news');

    // When the operator reveals and submits the "add section" form
    // (ScheduleEditor's form always exists; SectionsGrid's add form is
    // revealed by the "Aggiungi sezione" toggle and appears after it in
    // DOM order)
    await findButtonByText(wrapper, 'Aggiungi sezione').trigger('click');
    const forms = wrapper.findAll('form');
    const sectionForm = forms[forms.length - 1];
    if (!sectionForm) throw new Error('add-section form not found');
    await sectionForm.find('input[type="text"]').setValue('news');
    await sectionForm.trigger('submit');
    await vi.advanceTimersByTimeAsync(0);

    // Then it appears as a card in the grid
    expect(getConfigSpy).toHaveBeenCalledTimes(2);
    expect(wrapper.text()).toContain('news');

    // When the operator clicks through to the section's detail page
    await wrapper.find('a[href="/ingest/sections/1"]').trigger('click');
    await flushPromises();
    await vi.advanceTimersByTimeAsync(0);

    expect(wrapper.find('h1').text()).toBe('news');

    // And adds a scrape source there (SourceList's form is the first on the
    // detail page, before UploadDropzone's own form)
    const sourceForm = wrapper.find('form');
    await sourceForm
      .find('input[type="text"]')
      .setValue('https://example.com/news');
    await sourceForm.trigger('submit');
    await vi.advanceTimersByTimeAsync(0);

    // Then it appears in the section's source list
    expect(wrapper.text()).toContain('https://example.com/news');

    // When the operator navigates back and triggers a run
    await router.push('/ingest');
    await flushPromises();
    await vi.advanceTimersByTimeAsync(0);
    await findButtonByText(wrapper, 'Esegui ora').trigger('click');
    await vi.advanceTimersByTimeAsync(0);
    expect(wrapper.text()).toContain('pending');

    // Then the status renders pending then done
    await vi.advanceTimersByTimeAsync(2000);
    expect(wrapper.text()).toContain('done');
  });
});
