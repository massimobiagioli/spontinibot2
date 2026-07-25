import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';

import DsInput from '../DsInput.vue';

describe('DsInput', () => {
  it('associates the label with the input via a matching id', () => {
    const wrapper = mount(DsInput, {
      props: { modelValue: '', label: 'Domanda' },
    });

    const label = wrapper.get('label');
    const input = wrapper.get('input');

    expect(label.attributes('for')).toBe(input.attributes('id'));
    expect(label.text()).toBe('Domanda');
  });

  it('emits update:modelValue on input', async () => {
    const wrapper = mount(DsInput, {
      props: { modelValue: '', label: 'Domanda' },
    });

    await wrapper.get('input').setValue("A che ora apre l'anagrafe?");

    expect(wrapper.emitted('update:modelValue')?.[0]).toEqual([
      "A che ora apre l'anagrafe?",
    ]);
  });

  it('links the hint text via aria-describedby when provided', () => {
    const wrapper = mount(DsInput, {
      props: {
        modelValue: '',
        label: 'Domanda',
        hint: 'Scrivi la tua domanda al Comune.',
      },
    });

    const input = wrapper.get('input');
    const describedBy = input.attributes('aria-describedby');

    expect(describedBy).toBeTruthy();
    expect(wrapper.get(`#${describedBy}`).text()).toBe(
      'Scrivi la tua domanda al Comune.',
    );
  });

  it('passes the placeholder through to the native input', () => {
    const wrapper = mount(DsInput, {
      props: {
        modelValue: '',
        label: 'Domanda',
        placeholder: 'Scrivi la tua domanda…',
      },
    });

    expect(wrapper.get('input').attributes('placeholder')).toBe(
      'Scrivi la tua domanda…',
    );
  });
});
