import axe from 'axe-core';
import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import { createMemoryHistory, createRouter } from 'vue-router';

import App from '../App.vue';
import DevCatalog from '../views/DevCatalog.vue';
import HomeView from '../views/HomeView.vue';

function makeRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/', component: HomeView },
      { path: '/dev', component: DevCatalog },
    ],
  });
}

// jsdom cannot compute real layout/contrast, so this is a fast first-pass
// signal; pa11y (Task 4.2, real headless Chromium) is the authoritative gate.
async function runAxe(el: Element) {
  return axe.run(el, {
    rules: { 'color-contrast': { enabled: false } },
  });
}

describe('accessibility', () => {
  it('the app shell has zero axe violations', async () => {
    const router = makeRouter();
    await router.push('/');
    await router.isReady();

    const wrapper = mount(App, {
      global: { plugins: [router] },
      attachTo: document.body,
    });

    const results = await runAxe(wrapper.element);

    expect(results.violations).toEqual([]);

    wrapper.unmount();
  });

  it('the /dev catalog has zero axe violations', async () => {
    const router = makeRouter();
    await router.push('/dev');
    await router.isReady();

    const wrapper = mount(App, {
      global: { plugins: [router] },
      attachTo: document.body,
    });

    const results = await runAxe(wrapper.element);

    expect(results.violations).toEqual([]);

    wrapper.unmount();
  });
});
