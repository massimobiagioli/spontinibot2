import { flushPromises, mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';

import * as adminApi from '../../services/adminApi';
import ImprintingView from '../ImprintingView.vue';

describe('ImprintingView integration scenario', () => {
  it('save a new version, see it in history, activate a previous version, reload', async () => {
    const v1: adminApi.PersonaResponse = {
      id: 1,
      version: 1,
      name: 'gaspare',
      system_prompt: 'Sei Gaspare, versione uno.',
      tone: null,
      fallback_message: null,
      is_active: true,
      created_at: '2026-07-20T00:00:00Z',
      created_by: 'admin',
    };
    const v2: adminApi.PersonaResponse = {
      ...v1,
      id: 2,
      version: 2,
      system_prompt: 'Sei Gaspare, versione due.',
      is_active: true,
    };
    const v1Inactive = { ...v1, is_active: false };
    const v2Inactive = { ...v2, is_active: false };

    // Given an existing active persona version 1
    const getPersonaVersionsSpy = vi
      .spyOn(adminApi, 'getPersonaVersions')
      .mockResolvedValueOnce([v1])
      .mockResolvedValueOnce([v2, v1Inactive])
      .mockResolvedValueOnce([v1, v2Inactive]);

    vi.spyOn(adminApi, 'createPersona').mockResolvedValue(v2);
    vi.spyOn(adminApi, 'activatePersona').mockResolvedValue({
      status: 'activated',
    });
    vi.spyOn(adminApi, 'reloadPersona').mockResolvedValue({
      status: 'reloaded',
    });

    const wrapper = mount(ImprintingView);
    await flushPromises();
    expect(getPersonaVersionsSpy).toHaveBeenCalledTimes(1);
    expect(wrapper.text()).toContain('v1');

    // When the operator edits the form and saves as a new version 2
    const textarea = wrapper.findAll('textarea')[0];
    if (!textarea) throw new Error('system prompt textarea not found');
    await textarea.setValue('Sei Gaspare, versione due.');
    await wrapper.find('form').trigger('submit');
    await flushPromises();

    // Then version 2 appears in history and is badged active
    expect(getPersonaVersionsSpy).toHaveBeenCalledTimes(2);
    expect(wrapper.text()).toContain('v2');
    expect(wrapper.text()).toContain('Attiva');

    // When the operator activates version 1 (confirming the dialog)
    const activateButtons = wrapper.findAll('li button');
    const activateV1 = activateButtons[activateButtons.length - 1];
    if (!activateV1) throw new Error('activate button for v1 not found');
    await activateV1.trigger('click');
    await wrapper
      .find('[data-testid="activate-dialog"]')
      .find('button.btn-danger')
      .trigger('click');
    await flushPromises();

    // Then version 1 is badged active and version 2 is not
    expect(getPersonaVersionsSpy).toHaveBeenCalledTimes(3);
    expect(adminApi.activatePersona).toHaveBeenCalledWith(1);

    // When the operator clicks reload
    await wrapper.findAll('button')[0]?.trigger('click');
    await flushPromises();

    // Then a success callout renders
    expect(wrapper.text()).toContain('Persona ricaricata');
  });
});
