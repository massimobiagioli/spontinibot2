import { mount } from '@vue/test-utils';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import * as adminApi from '../../services/adminApi';
import IngestView from '../IngestView.vue';

describe('IngestView integration scenario', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('add a section, add a scraper source, trigger a run, see the run status', async () => {
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
      .mockResolvedValueOnce({
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

    const wrapper = mount(IngestView);
    await vi.advanceTimersByTimeAsync(0);
    expect(getConfigSpy).toHaveBeenCalledTimes(1);
    expect(wrapper.text()).not.toContain('news');

    // When the operator adds a section "news"
    // (with no sections yet, the only forms are ScheduleEditor's and
    // SectionList's "add section" form, in that DOM order)
    const initialForms = wrapper.findAll('form');
    const sectionForm = initialForms[initialForms.length - 1];
    if (!sectionForm) throw new Error('add-section form not found');
    await sectionForm.find('input[type="text"]').setValue('news');
    await sectionForm.trigger('submit');
    await vi.advanceTimersByTimeAsync(0);

    // Then it appears in the list
    expect(getConfigSpy).toHaveBeenCalledTimes(2);
    expect(wrapper.text()).toContain('news');

    // When the operator adds a scrape source to it
    // (SourceList's "add source" form is the first form nested in the
    // section's <li>, before UploadDropzone's own form)
    const sourceForm = wrapper.find('li form');
    await sourceForm
      .find('input[type="text"]')
      .setValue('https://example.com/news');
    await sourceForm.trigger('submit');
    await vi.advanceTimersByTimeAsync(0);

    // Then it appears in the list
    expect(getConfigSpy).toHaveBeenCalledTimes(3);
    expect(wrapper.text()).toContain('https://example.com/news');

    // When the operator triggers a run
    await wrapper.find('button').trigger('click');
    await vi.advanceTimersByTimeAsync(0);
    expect(wrapper.text()).toContain('pending');

    // Then the status renders pending then done
    await vi.advanceTimersByTimeAsync(2000);
    expect(wrapper.text()).toContain('done');
  });
});
