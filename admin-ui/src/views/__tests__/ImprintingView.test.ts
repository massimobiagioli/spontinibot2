import { flushPromises, mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';

import * as adminApi from '../../services/adminApi';
import ImprintingView from '../ImprintingView.vue';

describe('ImprintingView', () => {
  it('renders a loading state and then the active persona summary', async () => {
    const getPersonaVersionsSpy = vi
      .spyOn(adminApi, 'getPersonaVersions')
      .mockResolvedValue([
        {
          id: 1,
          version: 1,
          name: 'gaspare',
          system_prompt: 'Sei Gaspare.',
          tone: null,
          fallback_message: null,
          is_active: true,
          created_at: 'now',
          created_by: 'admin',
        },
      ]);

    const wrapper = mount(ImprintingView);

    expect(wrapper.text()).toContain('Caricamento della persona');

    await flushPromises();

    expect(getPersonaVersionsSpy).toHaveBeenCalledWith('gaspare');
    expect(wrapper.text()).not.toContain('Caricamento della persona');
    expect(wrapper.text()).toContain('v1');
  });

  it('renders an honest empty state when no persona version exists yet', async () => {
    vi.spyOn(adminApi, 'getPersonaVersions').mockResolvedValue([]);

    const wrapper = mount(ImprintingView);
    await flushPromises();

    expect(wrapper.text()).toContain('Nessuna persona configurata');
  });

  it('renders an honest error state when the fetch fails', async () => {
    vi.spyOn(adminApi, 'getPersonaVersions').mockRejectedValue(
      new adminApi.AdminApiError(401, 'invalid or missing X-Admin-Key header'),
    );

    const wrapper = mount(ImprintingView);
    await flushPromises();

    expect(wrapper.text()).toContain('invalid or missing X-Admin-Key header');
  });
});
