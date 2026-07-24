import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';

import DsConfirmDialog from '../DsConfirmDialog.vue';

describe('DsConfirmDialog', () => {
  it('renders the message and is open when the open prop is true', () => {
    const wrapper = mount(DsConfirmDialog, {
      props: { open: true, message: 'Eliminare la sezione "news"?' },
    });

    expect(wrapper.text()).toContain('Eliminare la sezione "news"?');
    expect(wrapper.find('dialog').element.open).toBe(true);
  });

  it('is not open when the open prop is false', () => {
    const wrapper = mount(DsConfirmDialog, {
      props: { open: false, message: 'Eliminare?' },
    });

    expect(wrapper.find('dialog').element.open).toBe(false);
  });

  it('emits confirm when the confirm button is clicked', async () => {
    const wrapper = mount(DsConfirmDialog, {
      props: { open: true, message: 'Eliminare?', confirmLabel: 'Elimina' },
    });

    await wrapper.find('button.btn-danger').trigger('click');

    expect(wrapper.emitted('confirm')).toHaveLength(1);
    expect(wrapper.find('button.btn-danger').text()).toBe('Elimina');
  });

  it('emits cancel when the cancel button is clicked', async () => {
    const wrapper = mount(DsConfirmDialog, {
      props: { open: true, message: 'Eliminare?' },
    });

    await wrapper.find('button.btn-outline-secondary').trigger('click');

    expect(wrapper.emitted('cancel')).toHaveLength(1);
  });

  it('emits cancel when the dialog is dismissed via the cancel event (Esc)', async () => {
    const wrapper = mount(DsConfirmDialog, {
      props: { open: true, message: 'Eliminare?' },
    });

    await wrapper.find('dialog').trigger('cancel');

    expect(wrapper.emitted('cancel')).toHaveLength(1);
  });
});
