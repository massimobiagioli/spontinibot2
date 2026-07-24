import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';

import DsButton from '../DsButton.vue';

describe('DsButton', () => {
  it('applies the primary variant class by default', () => {
    const wrapper = mount(DsButton, { slots: { default: 'Invia' } });

    expect(wrapper.classes()).toContain('btn');
    expect(wrapper.classes()).toContain('btn-primary');
    expect(wrapper.text()).toBe('Invia');
  });

  it('applies an outline class for the given variant', () => {
    const wrapper = mount(DsButton, {
      props: { variant: 'danger', outline: true },
    });

    expect(wrapper.classes()).toContain('btn-outline-danger');
    expect(wrapper.classes()).not.toContain('btn-danger');
  });

  it('emits click when pressed', async () => {
    const wrapper = mount(DsButton);

    await wrapper.trigger('click');

    expect(wrapper.emitted('click')).toHaveLength(1);
  });

  it('is disabled when the disabled prop is set', () => {
    const wrapper = mount(DsButton, { props: { disabled: true } });

    expect(wrapper.attributes('disabled')).toBeDefined();
  });
});
