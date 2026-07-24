import { flushPromises, mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';

import * as adminApi from '../../../services/adminApi';
import PersonaEditor from '../PersonaEditor.vue';

const activeVersion: adminApi.PersonaResponse = {
  id: 1,
  version: 1,
  name: 'gaspare',
  system_prompt: 'Sei Gaspare, compositore di Maiolati.',
  tone: 'solenne',
  fallback_message: 'Non ho trovato informazioni.',
  is_active: true,
  created_at: '2026-07-24T00:00:00Z',
  created_by: 'admin',
};

describe('PersonaEditor', () => {
  it('prefills the form from the active version', () => {
    const wrapper = mount(PersonaEditor, {
      props: { personaName: 'gaspare', activeVersion, hasAnyVersion: true },
    });

    expect(
      (wrapper.findAll('textarea')[0]?.element as HTMLTextAreaElement).value,
    ).toBe('Sei Gaspare, compositore di Maiolati.');
    const nameInput = wrapper.find('input[type="text"]')
      .element as HTMLInputElement;
    expect(nameInput.value).toBe('gaspare');
    expect(nameInput.disabled).toBe(true);
  });

  it('renders blank editable fields when there is no version yet', () => {
    const wrapper = mount(PersonaEditor, {
      props: {
        personaName: 'gaspare',
        activeVersion: null,
        hasAnyVersion: false,
      },
    });

    const nameInput = wrapper.find('input[type="text"]')
      .element as HTMLInputElement;
    expect(nameInput.value).toBe('gaspare');
    expect(nameInput.disabled).toBe(false);
    expect(
      (wrapper.findAll('textarea')[0]?.element as HTMLTextAreaElement).value,
    ).toBe('');
  });

  it('submits calls createPersona with the form payload and shows the success callout', async () => {
    const createPersonaSpy = vi
      .spyOn(adminApi, 'createPersona')
      .mockResolvedValue({ ...activeVersion, id: 2, version: 2 });

    const wrapper = mount(PersonaEditor, {
      props: { personaName: 'gaspare', activeVersion, hasAnyVersion: true },
    });

    await wrapper.findAll('textarea')[0]?.setValue('Nuovo prompt.');
    await wrapper.find('form').trigger('submit');
    await flushPromises();

    expect(createPersonaSpy).toHaveBeenCalledWith({
      name: 'gaspare',
      system_prompt: 'Nuovo prompt.',
      tone: 'solenne',
      fallback_message: 'Non ho trovato informazioni.',
      activate: true,
    });
    expect(wrapper.text()).toContain('Versione salvata');
    expect(wrapper.emitted('saved')).toBeTruthy();
  });

  it('shows the honest error message from AdminApiError on failure', async () => {
    vi.spyOn(adminApi, 'createPersona').mockRejectedValue(
      new adminApi.AdminApiError(500, 'internal error'),
    );

    const wrapper = mount(PersonaEditor, {
      props: { personaName: 'gaspare', activeVersion, hasAnyVersion: true },
    });

    await wrapper.find('form').trigger('submit');
    await flushPromises();

    expect(wrapper.text()).toContain('internal error');
  });
});
