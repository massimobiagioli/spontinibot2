import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';

import DsInfoButton from '../DsInfoButton.vue';

describe('DsInfoButton', () => {
  it('is closed by default', () => {
    const wrapper = mount(DsInfoButton, {
      props: { title: 'Formato finestra temporale' },
      slots: { default: 'Esempi: "30d", "2026-07".' },
    });

    expect(wrapper.find('dialog').element.open).toBe(false);
  });

  it('opens the dialog with the title and slotted content when the info button is clicked', async () => {
    const wrapper = mount(DsInfoButton, {
      props: { title: 'Formato finestra temporale' },
      slots: { default: 'Esempi: "30d", "2026-07".' },
    });

    await wrapper.find('button.ds-info-button').trigger('click');

    expect(wrapper.find('dialog').element.open).toBe(true);
    expect(wrapper.text()).toContain('Formato finestra temporale');
    expect(wrapper.text()).toContain('Esempi: "30d", "2026-07".');
  });

  it('closes the dialog when the footer close button is clicked', async () => {
    const wrapper = mount(DsInfoButton, {
      props: { title: 'Formato finestra temporale' },
    });

    await wrapper.find('button.ds-info-button').trigger('click');
    await wrapper.find('.ds-info-dialog__footer button').trigger('click');

    expect(wrapper.find('dialog').element.open).toBe(false);
  });

  it('closes when the dialog is dismissed via the cancel event (Esc)', async () => {
    const wrapper = mount(DsInfoButton, {
      props: { title: 'Formato finestra temporale' },
    });

    await wrapper.find('button.ds-info-button').trigger('click');
    await wrapper.find('dialog').trigger('cancel');

    expect(wrapper.find('dialog').element.open).toBe(false);
  });
});
