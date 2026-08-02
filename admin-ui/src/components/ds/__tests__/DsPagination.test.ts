import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';

import DsPagination from '../DsPagination.vue';

describe('DsPagination', () => {
  it('renders nothing when there is only one page', () => {
    const wrapper = mount(DsPagination, {
      props: { currentPage: 1, totalPages: 1, label: 'Paginazione test' },
    });

    expect(wrapper.find('nav').exists()).toBe(false);
  });

  it('renders a nav labeled by the given aria-label, one page-item per page plus prev/next', () => {
    const wrapper = mount(DsPagination, {
      props: { currentPage: 2, totalPages: 3, label: 'Paginazione test' },
    });

    const nav = wrapper.get('nav[aria-label="Paginazione test"]');
    expect(nav.findAll('li.page-item')).toHaveLength(5); // prev + 3 pages + next
  });

  it('marks the current page with aria-current="page"', () => {
    const wrapper = mount(DsPagination, {
      props: { currentPage: 2, totalPages: 3, label: 'Paginazione test' },
    });

    const buttons = wrapper.findAll('li.page-item button');
    expect(buttons[2]?.attributes('aria-current')).toBe('page');
    expect(buttons[1]?.attributes('aria-current')).toBeUndefined();
  });

  it('disables the prev button on the first page and the next button on the last page', () => {
    const first = mount(DsPagination, {
      props: { currentPage: 1, totalPages: 3, label: 'Paginazione test' },
    });
    const firstButtons = first.findAll('li.page-item button');
    expect(firstButtons[0]?.attributes('disabled')).toBeDefined();
    expect(
      firstButtons[firstButtons.length - 1]?.attributes('disabled'),
    ).toBeUndefined();

    const last = mount(DsPagination, {
      props: { currentPage: 3, totalPages: 3, label: 'Paginazione test' },
    });
    const lastButtons = last.findAll('li.page-item button');
    expect(lastButtons[0]?.attributes('disabled')).toBeUndefined();
    expect(
      lastButtons[lastButtons.length - 1]?.attributes('disabled'),
    ).toBeDefined();
  });

  it('emits update:currentPage when a page number is clicked', async () => {
    const wrapper = mount(DsPagination, {
      props: { currentPage: 1, totalPages: 3, label: 'Paginazione test' },
    });

    const buttons = wrapper.findAll('li.page-item button');
    await buttons[2]!.trigger('click'); // page "2"

    expect(wrapper.emitted('update:currentPage')).toEqual([[2]]);
  });

  it('emits update:currentPage when next/prev are clicked', async () => {
    const wrapper = mount(DsPagination, {
      props: { currentPage: 2, totalPages: 3, label: 'Paginazione test' },
    });

    const buttons = wrapper.findAll('li.page-item button');
    await buttons[0]!.trigger('click'); // prev
    await buttons[buttons.length - 1]!.trigger('click'); // next

    expect(wrapper.emitted('update:currentPage')).toEqual([[1], [3]]);
  });
});
