import { mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';

import * as adminApi from '../../../services/adminApi';
import SectionList from '../SectionList.vue';

function section(overrides: Partial<adminApi.IngestSectionWithSources> = {}) {
  return {
    id: 1,
    name: 'news',
    ordering: 10,
    created_at: '2026-07-24T00:00:00Z',
    sources: [],
    ...overrides,
  };
}

describe('SectionList', () => {
  it('renders each section name and ordering', () => {
    const wrapper = mount(SectionList, {
      props: {
        sections: [
          section({ name: 'news' }),
          section({ id: 2, name: 'sport' }),
        ],
      },
    });

    expect(wrapper.text()).toContain('news');
    expect(wrapper.text()).toContain('sport');
  });

  it('adding a section calls createSection and emits changed', async () => {
    const createSpy = vi
      .spyOn(adminApi, 'createSection')
      .mockResolvedValue(section({ id: 3, name: 'delibere' }));

    const wrapper = mount(SectionList, { props: { sections: [] } });

    await wrapper.find('input[type="text"]').setValue('delibere');
    await wrapper.find('form').trigger('submit');
    await Promise.resolve();
    await wrapper.vm.$nextTick();

    expect(createSpy).toHaveBeenCalledWith('delibere', 10);
    expect(wrapper.emitted('changed')).toHaveLength(1);
  });

  it('deleting a section requires confirmation before calling deleteSection', async () => {
    const deleteSpy = vi
      .spyOn(adminApi, 'deleteSection')
      .mockResolvedValue(true);

    const wrapper = mount(SectionList, {
      props: { sections: [section()] },
    });

    await wrapper.find('li > button').trigger('click');
    await wrapper.vm.$nextTick();

    expect(deleteSpy).not.toHaveBeenCalled();
    const dialog = wrapper.find<HTMLDialogElement>(
      '[data-testid="section-delete-dialog"]',
    );
    expect(dialog.element.open).toBe(true);

    await dialog.find('button.btn-danger').trigger('click');
    await Promise.resolve();
    await wrapper.vm.$nextTick();

    expect(deleteSpy).toHaveBeenCalledWith(1);
    expect(wrapper.emitted('changed')).toHaveLength(1);
  });

  it('cancelling the delete confirmation does not call deleteSection', async () => {
    const deleteSpy = vi
      .spyOn(adminApi, 'deleteSection')
      .mockResolvedValue(true);

    const wrapper = mount(SectionList, {
      props: { sections: [section()] },
    });

    await wrapper.find('li > button').trigger('click');
    await wrapper.vm.$nextTick();
    const dialog = wrapper.find<HTMLDialogElement>(
      '[data-testid="section-delete-dialog"]',
    );
    await dialog.find('button.btn-outline-secondary').trigger('click');
    await wrapper.vm.$nextTick();

    expect(deleteSpy).not.toHaveBeenCalled();
    expect(dialog.element.open).toBe(false);
  });
});
