import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';

import DsInput from '../DsInput.vue';

describe('DsInput', () => {
  it('associates the label with the input via a matching id', () => {
    const wrapper = mount(DsInput, {
      props: { modelValue: '', label: 'Nome' },
    });

    const label = wrapper.get('label');
    const input = wrapper.get('input');

    expect(label.attributes('for')).toBe(input.attributes('id'));
    expect(label.text()).toBe('Nome');
  });

  it('emits update:modelValue on input', async () => {
    const wrapper = mount(DsInput, {
      props: { modelValue: '', label: 'Nome' },
    });

    await wrapper.get('input').setValue('Mario');

    expect(wrapper.emitted('update:modelValue')?.[0]).toEqual(['Mario']);
  });

  it('links the hint text via aria-describedby when provided', () => {
    const wrapper = mount(DsInput, {
      props: {
        modelValue: '',
        label: 'Nome',
        hint: 'Il tuo nome di battesimo',
      },
    });

    const input = wrapper.get('input');
    const describedBy = input.attributes('aria-describedby');

    expect(describedBy).toBeTruthy();
    expect(wrapper.get(`#${describedBy}`).text()).toBe(
      'Il tuo nome di battesimo',
    );
  });
});
