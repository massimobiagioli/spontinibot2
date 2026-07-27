import { mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';

import * as adminApi from '../../../services/adminApi';
import ManualIngestForm from '../ManualIngestForm.vue';

async function fillForm(wrapper: ReturnType<typeof mount>): Promise<void> {
  const inputs = wrapper.findAll('input');
  await inputs[0]!.setValue('storia');
  await inputs[1]!.setValue('https://it.wikipedia.org/wiki/Maiolati_Spontini');
  await inputs[2]!.setValue('30d');
}

describe('ManualIngestForm', () => {
  it('submits the form and renders the success result', async () => {
    const triggerSpy = vi
      .spyOn(adminApi, 'triggerManualIngest')
      .mockResolvedValue({
        section: 'storia',
        src: 'https://it.wikipedia.org/wiki/Maiolati_Spontini',
        window: '30d',
        status: 'ingested',
      });

    const wrapper = mount(ManualIngestForm);
    await fillForm(wrapper);
    await wrapper.find('form').trigger('submit');
    await vi.waitFor(() => expect(wrapper.text()).toContain('Ingest completato'));

    expect(triggerSpy).toHaveBeenCalledWith(
      'storia',
      'https://it.wikipedia.org/wiki/Maiolati_Spontini',
      '30d',
    );
    expect(wrapper.text()).toContain('storia');
  });

  it('shows an honest error message when the request fails', async () => {
    vi.spyOn(adminApi, 'triggerManualIngest').mockRejectedValue(
      new adminApi.AdminApiError(403, 'robots.txt: disallows scraping'),
    );

    const wrapper = mount(ManualIngestForm);
    await fillForm(wrapper);
    await wrapper.find('form').trigger('submit');
    await vi.waitFor(() =>
      expect(wrapper.text()).toContain('robots.txt: disallows scraping'),
    );
  });

  it('disables the submit button until every field is filled', async () => {
    const wrapper = mount(ManualIngestForm);
    const button = wrapper.find('button');
    expect(button.attributes('disabled')).toBeDefined();

    await fillForm(wrapper);
    expect(button.attributes('disabled')).toBeUndefined();
  });
});
