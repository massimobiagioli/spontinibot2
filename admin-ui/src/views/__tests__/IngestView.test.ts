import { flushPromises, mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';

import * as adminApi from '../../services/adminApi';
import IngestView from '../IngestView.vue';

describe('IngestView', () => {
  it('renders a loading state and then the resolved configuration', async () => {
    const getIngestConfigSpy = vi
      .spyOn(adminApi, 'getIngestConfig')
      .mockResolvedValue({
        schedule: null,
        sections: [
          { id: 1, name: 'news', ordering: 10, created_at: 'now', sources: [] },
        ],
      });

    const wrapper = mount(IngestView);

    expect(wrapper.text()).toContain('Caricamento della configurazione');

    await flushPromises();

    expect(getIngestConfigSpy).toHaveBeenCalledOnce();
    expect(wrapper.text()).not.toContain('Caricamento della configurazione');
    expect(wrapper.text()).toContain('news');
  });

  it('renders an honest error state when the config fetch fails', async () => {
    vi.spyOn(adminApi, 'getIngestConfig').mockRejectedValue(
      new adminApi.AdminApiError(401, 'invalid or missing session cookie'),
    );

    const wrapper = mount(IngestView);
    await flushPromises();

    expect(wrapper.text()).toContain('invalid or missing session cookie');
  });
});
