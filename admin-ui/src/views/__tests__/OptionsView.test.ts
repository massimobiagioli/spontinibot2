import { flushPromises, mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';

import * as adminApi from '../../services/adminApi';
import OptionsView from '../OptionsView.vue';

describe('OptionsView', () => {
  it('renders the page title and the Scraper section', async () => {
    vi.spyOn(adminApi, 'listRobotsBypassHosts').mockResolvedValue([]);

    const wrapper = mount(OptionsView);
    await flushPromises();

    expect(wrapper.get('h1').text()).toBe('Opzioni');
    expect(wrapper.get('h2').text()).toBe('Scraper');
  });
});
