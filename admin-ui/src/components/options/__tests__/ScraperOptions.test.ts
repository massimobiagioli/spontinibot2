import { flushPromises, mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';

import * as adminApi from '../../../services/adminApi';
import ScraperOptions from '../ScraperOptions.vue';

const sampleHosts: adminApi.RobotsBypassHostResponse[] = [
  {
    id: 1,
    host: 'a.example.com',
    created_at: '2026-08-02 00:00:00',
  },
  {
    id: 2,
    host: 'b.example.com',
    created_at: '2026-08-02 00:00:00',
  },
];

describe('ScraperOptions', () => {
  it('loads the current hosts and shows them one per line in the textarea', async () => {
    vi.spyOn(adminApi, 'listRobotsBypassHosts').mockResolvedValue(sampleHosts);

    const wrapper = mount(ScraperOptions);
    expect(wrapper.text()).toContain('Caricamento');

    await flushPromises();

    const textarea = wrapper.get('textarea').element as HTMLTextAreaElement;
    expect(textarea.value).toBe('a.example.com\nb.example.com');
  });

  it('shows the honest error message from AdminApiError when loading fails', async () => {
    vi.spyOn(adminApi, 'listRobotsBypassHosts').mockRejectedValue(
      new adminApi.AdminApiError(500, 'internal error'),
    );

    const wrapper = mount(ScraperOptions);
    await flushPromises();

    expect(wrapper.text()).toContain('internal error');
    expect(wrapper.find('textarea').exists()).toBe(false);
  });

  it('saves the edited textarea content and shows a confirmation', async () => {
    vi.spyOn(adminApi, 'listRobotsBypassHosts').mockResolvedValue(sampleHosts);
    const replaceSpy = vi
      .spyOn(adminApi, 'replaceRobotsBypassHosts')
      .mockResolvedValue([
        { id: 3, host: 'c.example.com', created_at: '2026-08-02 00:00:00' },
      ]);

    const wrapper = mount(ScraperOptions);
    await flushPromises();

    await wrapper.get('textarea').setValue('c.example.com');
    await wrapper.get('form').trigger('submit');
    await flushPromises();

    expect(replaceSpy).toHaveBeenCalledWith('c.example.com');
    expect(wrapper.text()).toContain('Elenco salvato');
    const textarea = wrapper.get('textarea').element as HTMLTextAreaElement;
    expect(textarea.value).toBe('c.example.com');
  });

  it('shows the honest error message from AdminApiError when saving fails', async () => {
    vi.spyOn(adminApi, 'listRobotsBypassHosts').mockResolvedValue(sampleHosts);
    vi.spyOn(adminApi, 'replaceRobotsBypassHosts').mockRejectedValue(
      new adminApi.AdminApiError(400, 'host non valido'),
    );

    const wrapper = mount(ScraperOptions);
    await flushPromises();

    await wrapper.get('form').trigger('submit');
    await flushPromises();

    expect(wrapper.text()).toContain('host non valido');
    expect(wrapper.text()).not.toContain('Elenco salvato');
  });

  it('hides the saved confirmation again once the textarea is edited', async () => {
    vi.spyOn(adminApi, 'listRobotsBypassHosts').mockResolvedValue(sampleHosts);
    vi.spyOn(adminApi, 'replaceRobotsBypassHosts').mockResolvedValue(
      sampleHosts,
    );

    const wrapper = mount(ScraperOptions);
    await flushPromises();

    await wrapper.get('form').trigger('submit');
    await flushPromises();
    expect(wrapper.text()).toContain('Elenco salvato');

    await wrapper.get('textarea').trigger('input');
    expect(wrapper.text()).not.toContain('Elenco salvato');
  });
});
