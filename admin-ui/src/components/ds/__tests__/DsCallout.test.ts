import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';

import DsCallout from '../DsCallout.vue';

describe('DsCallout', () => {
  it('applies the primary variant class by default and renders the body', () => {
    const wrapper = mount(DsCallout, {
      slots: { default: 'Testo del callout' },
    });

    expect(wrapper.classes()).toContain('callout');
    expect(wrapper.classes()).toContain('callout-primary');
    expect(wrapper.text()).toContain('Testo del callout');
  });

  it('renders a title when provided', () => {
    const wrapper = mount(DsCallout, {
      props: { title: 'Attenzione', variant: 'warning' },
    });

    expect(wrapper.classes()).toContain('callout-warning');
    expect(wrapper.get('.callout-title').text()).toBe('Attenzione');
  });

  it('applies the highlight class when requested', () => {
    const wrapper = mount(DsCallout, { props: { highlight: true } });

    expect(wrapper.classes()).toContain('callout-highlight');
  });
});
