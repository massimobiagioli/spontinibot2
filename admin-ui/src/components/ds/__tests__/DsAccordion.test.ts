import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';

import DsAccordion from '../DsAccordion.vue';

describe('DsAccordion', () => {
  it('renders the title as the collapsed summary and is closed by default', () => {
    const wrapper = mount(DsAccordion, {
      props: { title: 'Pianificazione' },
      slots: { default: 'Contenuto del pannello' },
    });

    expect(wrapper.find('summary').text()).toBe('Pianificazione');
    expect(wrapper.find('details').element.open).toBe(false);
    // Native <details> keeps its content in the DOM even while collapsed —
    // it's the rendering, not the presence, that's hidden.
    expect(wrapper.text()).toContain('Contenuto del pannello');
  });

  it('opens by default when defaultOpen is set', () => {
    const wrapper = mount(DsAccordion, {
      props: { title: 'Sezioni', defaultOpen: true },
    });

    expect(wrapper.find('details').element.open).toBe(true);
  });
});
