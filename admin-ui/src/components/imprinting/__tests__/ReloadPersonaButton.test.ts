import { flushPromises, mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';

import * as adminApi from '../../../services/adminApi';
import ReloadPersonaButton from '../ReloadPersonaButton.vue';

describe('ReloadPersonaButton', () => {
  it('calls reloadPersona on click and shows the success callout', async () => {
    const reloadPersonaSpy = vi
      .spyOn(adminApi, 'reloadPersona')
      .mockResolvedValue({ status: 'reloaded' });

    const wrapper = mount(ReloadPersonaButton);
    await wrapper.find('button').trigger('click');
    await flushPromises();

    expect(reloadPersonaSpy).toHaveBeenCalledOnce();
    expect(wrapper.text()).toContain('Persona ricaricata');
  });

  it('shows the honest error message from AdminApiError on failure', async () => {
    vi.spyOn(adminApi, 'reloadPersona').mockRejectedValue(
      new adminApi.AdminApiError(500, 'internal error'),
    );

    const wrapper = mount(ReloadPersonaButton);
    await wrapper.find('button').trigger('click');
    await flushPromises();

    expect(wrapper.text()).toContain('internal error');
  });
});
