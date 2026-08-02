import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import { createMemoryHistory, createRouter } from 'vue-router';

import DevCatalog from '../DevCatalog.vue';

describe('DevCatalog', () => {
  it('mounts without error and lists every wrapper component', async () => {
    const router = createRouter({
      history: createMemoryHistory(),
      routes: [
        { path: '/', component: { template: '<div />' } },
        { path: '/dev', component: DevCatalog },
      ],
    });
    await router.push('/dev');
    await router.isReady();

    const wrapper = mount(DevCatalog, { global: { plugins: [router] } });

    expect(wrapper.find('h1').text()).toBe('Catalogo componenti');
    expect(wrapper.findAll('h2').map((h) => h.text())).toEqual([
      'DsButton',
      'DsInput',
      'DsCallout',
      'DsInfoButton',
      'Titolo della dialog',
      'DsAccordion',
      'Primo pannello',
      'Secondo pannello',
      'DsPagination',
      'DsNav',
    ]);
  });
});
