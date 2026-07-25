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

  it('uses an assertive alert role for the danger variant so it is announced without focus', () => {
    const wrapper = mount(DsCallout, { props: { variant: 'danger' } });

    expect(wrapper.attributes('role')).toBe('alert');
  });

  it('uses a polite status role for the success variant so it is announced without focus', () => {
    const wrapper = mount(DsCallout, { props: { variant: 'success' } });

    expect(wrapper.attributes('role')).toBe('status');
  });

  it('uses the static note role for the primary and warning variants', () => {
    expect(
      mount(DsCallout, { props: { variant: 'primary' } }).attributes('role'),
    ).toBe('note');
    expect(
      mount(DsCallout, { props: { variant: 'warning' } }).attributes('role'),
    ).toBe('note');
  });

  it('lets a consumer override the role for a dynamically-appearing callout that must be announced', () => {
    const wrapper = mount(DsCallout, {
      props: { variant: 'primary', role: 'status' },
    });

    expect(wrapper.attributes('role')).toBe('status');
    expect(wrapper.classes()).toContain('callout-primary');
  });
});
