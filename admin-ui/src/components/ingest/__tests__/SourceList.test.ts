import { mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';

import * as adminApi from '../../../services/adminApi';
import SourceList from '../SourceList.vue';

function source(overrides: Partial<adminApi.IngestSourceResponse> = {}) {
  return {
    id: 1,
    section_id: 10,
    source_type: 'scrape',
    url: 'https://example.com/news',
    enabled: true,
    created_at: '2026-07-24T00:00:00Z',
    coming_soon: false,
    ...overrides,
  };
}

describe('SourceList', () => {
  it('renders a scrape source as active', () => {
    const wrapper = mount(SourceList, {
      props: { sectionId: 10, sources: [source()] },
    });

    expect(wrapper.text()).toContain('https://example.com/news');
    expect(wrapper.find('li').classes()).not.toContain('text-muted');
  });

  it('renders a coming-soon api source as disabled with a tooltip', () => {
    const wrapper = mount(SourceList, {
      props: {
        sectionId: 10,
        sources: [
          source({
            id: 2,
            source_type: 'api',
            url: 'https://api.example.com',
            enabled: false,
            coming_soon: true,
          }),
        ],
      },
    });

    expect(wrapper.find('li').classes()).toContain('text-muted');
    expect(wrapper.text()).toContain('Prossimamente');
    expect(wrapper.find('li span').attributes('title')).toBeTruthy();
  });

  it('adding a scrape source calls createSource and emits changed', async () => {
    const createSpy = vi
      .spyOn(adminApi, 'createSource')
      .mockResolvedValue(source({ id: 3, url: 'https://example.com/new' }));

    const wrapper = mount(SourceList, {
      props: { sectionId: 10, sources: [] },
    });

    await wrapper
      .find('input[type="text"]')
      .setValue('https://example.com/new');
    await wrapper.find('form').trigger('submit');
    await Promise.resolve();
    await wrapper.vm.$nextTick();

    expect(createSpy).toHaveBeenCalledWith(
      10,
      'scrape',
      'https://example.com/new',
      true,
    );
    expect(wrapper.emitted('changed')).toHaveLength(1);
  });

  it('shows an honest error message when adding a source fails', async () => {
    vi.spyOn(adminApi, 'createSource').mockRejectedValue(
      new adminApi.AdminApiError(400, 'invalid source_type: bogus'),
    );

    const wrapper = mount(SourceList, {
      props: { sectionId: 10, sources: [] },
    });

    await wrapper
      .find('input[type="text"]')
      .setValue('https://example.com/new');
    await wrapper.find('form').trigger('submit');
    await Promise.resolve();
    await wrapper.vm.$nextTick();

    expect(wrapper.text()).toContain('invalid source_type: bogus');
    expect(wrapper.emitted('changed')).toBeUndefined();
  });

  it('deleting a source requires confirmation before calling deleteSource', async () => {
    const deleteSpy = vi
      .spyOn(adminApi, 'deleteSource')
      .mockResolvedValue(true);

    const wrapper = mount(SourceList, {
      props: { sectionId: 10, sources: [source()] },
    });

    await wrapper.find('li button').trigger('click');
    await wrapper.vm.$nextTick();

    expect(deleteSpy).not.toHaveBeenCalled();
    expect(wrapper.find('dialog').element.open).toBe(true);

    await wrapper.find('dialog button.btn-danger').trigger('click');
    await Promise.resolve();
    await wrapper.vm.$nextTick();

    expect(deleteSpy).toHaveBeenCalledWith(1);
    expect(wrapper.emitted('changed')).toHaveLength(1);
  });

  it('shows an honest error message when deleting a source fails', async () => {
    vi.spyOn(adminApi, 'deleteSource').mockRejectedValue(
      new adminApi.AdminApiError(500, 'database error: connection refused'),
    );

    const wrapper = mount(SourceList, {
      props: { sectionId: 10, sources: [source()] },
    });

    await wrapper.find('li button').trigger('click');
    await wrapper.vm.$nextTick();
    await wrapper.find('dialog button.btn-danger').trigger('click');
    await Promise.resolve();
    await wrapper.vm.$nextTick();

    expect(wrapper.text()).toContain('database error: connection refused');
    expect(wrapper.emitted('changed')).toBeUndefined();
    expect(wrapper.find('dialog').element.open).toBe(false);
  });
});
