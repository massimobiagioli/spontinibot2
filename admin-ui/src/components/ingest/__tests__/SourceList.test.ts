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

function curationSource(
  overrides: Partial<adminApi.CurationSourceResponse> = {},
) {
  return {
    source_url: 'https://www.halleyweb.com/.../delibere',
    last_item_date: '2026-07-13',
    updated_at: '2026-07-27 15:07:53',
    ...overrides,
  };
}

describe('SourceList', () => {
  it('renders a scrape source as active', () => {
    const wrapper = mount(SourceList, {
      props: { sectionId: 10, sources: [source()], curationSources: [] },
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
        curationSources: [],
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
      props: { sectionId: 10, sources: [], curationSources: [] },
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
      props: { sectionId: 10, sources: [], curationSources: [] },
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
      props: { sectionId: 10, sources: [source()], curationSources: [] },
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
      props: { sectionId: 10, sources: [source()], curationSources: [] },
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

  it('renders a curation source distinctly, with no delete action', () => {
    const wrapper = mount(SourceList, {
      props: {
        sectionId: 3,
        sources: [],
        curationSources: [curationSource()],
      },
    });

    expect(wrapper.text()).toContain('https://www.halleyweb.com/.../delibere');
    expect(wrapper.text()).toContain('Curazione automatica');
    expect(wrapper.text()).toContain('2026-07-13');
    expect(wrapper.find('.source-list__curation-item button').exists()).toBe(
      false,
    );
  });

  it('shows an honest empty state when there are no sources of any kind', () => {
    const wrapper = mount(SourceList, {
      props: { sectionId: 2, sources: [], curationSources: [] },
    });

    expect(wrapper.text()).toContain(
      'Nessuna fonte configurata per questa sezione.',
    );
  });

  it('does not show the empty state when a curation source exists', () => {
    const wrapper = mount(SourceList, {
      props: {
        sectionId: 3,
        sources: [],
        curationSources: [curationSource()],
      },
    });

    expect(wrapper.text()).not.toContain(
      'Nessuna fonte configurata per questa sezione.',
    );
  });

  it('does not show the empty state when an ordinary source exists', () => {
    const wrapper = mount(SourceList, {
      props: { sectionId: 10, sources: [source()], curationSources: [] },
    });

    expect(wrapper.text()).not.toContain(
      'Nessuna fonte configurata per questa sezione.',
    );
  });
});
