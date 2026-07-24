import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import { createMemoryHistory, createRouter } from 'vue-router';

import DsNav from '../DsNav.vue';

function makeRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/', name: 'home', component: { template: '<div />' } },
      { path: '/dev', name: 'dev', component: { template: '<div />' } },
    ],
  });
}

describe('DsNav', () => {
  it('renders one item per link', async () => {
    const router = makeRouter();
    await router.push('/');
    await router.isReady();

    const wrapper = mount(DsNav, {
      props: {
        links: [
          { to: '/', label: 'Home' },
          { to: '/dev', label: 'Catalogo componenti' },
          { label: 'Training' },
        ],
      },
      global: { plugins: [router] },
    });

    expect(wrapper.findAll('li')).toHaveLength(3);
    expect(wrapper.text()).toContain('Training');
  });

  it('marks the active route with aria-current="page"', async () => {
    const router = makeRouter();
    await router.push('/dev');
    await router.isReady();

    const wrapper = mount(DsNav, {
      props: {
        links: [
          { to: '/', label: 'Home' },
          { to: '/dev', label: 'Catalogo componenti' },
        ],
      },
      global: { plugins: [router] },
    });

    const activeLink = wrapper.get('a[aria-current="page"]');
    expect(activeLink.text()).toContain('Catalogo componenti');
  });

  it('renders a link with no route as a disabled, non-interactive item', async () => {
    const router = makeRouter();
    await router.push('/');
    await router.isReady();

    const wrapper = mount(DsNav, {
      props: { links: [{ label: 'Training' }] },
      global: { plugins: [router] },
    });

    expect(wrapper.find('a').exists()).toBe(false);
    expect(wrapper.get('span.disabled').attributes('aria-disabled')).toBe(
      'true',
    );
  });
});
