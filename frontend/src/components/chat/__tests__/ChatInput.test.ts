import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';

import ChatInput from '../ChatInput.vue';

describe('ChatInput', () => {
  it('emits "ask" with the trimmed question on submit and clears the input', async () => {
    const wrapper = mount(ChatInput, { props: { busy: false } });

    await wrapper.get('input').setValue("  Che orari ha l'ufficio?  ");
    await wrapper.get('form').trigger('submit');

    expect(wrapper.emitted('ask')).toEqual([["Che orari ha l'ufficio?"]]);
    expect((wrapper.get('input').element as HTMLInputElement).value).toBe('');
  });

  it('does not emit "ask" when the input is blank or whitespace-only', async () => {
    const wrapper = mount(ChatInput, { props: { busy: false } });

    await wrapper.get('input').setValue('   ');
    await wrapper.get('form').trigger('submit');

    expect(wrapper.emitted('ask')).toBeUndefined();
  });

  it('disables the submit button while busy', () => {
    const wrapper = mount(ChatInput, { props: { busy: true } });

    expect(wrapper.get('button').attributes('disabled')).toBeDefined();
  });

  it('enables the submit button when not busy', () => {
    const wrapper = mount(ChatInput, { props: { busy: false } });

    expect(wrapper.get('button').attributes('disabled')).toBeUndefined();
  });
});
