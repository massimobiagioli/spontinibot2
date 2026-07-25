import axe from 'axe-core';
import { flushPromises, mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
import { createMemoryHistory, createRouter } from 'vue-router';

import App from '../App.vue';
import DevCatalog from '../views/DevCatalog.vue';
import HomeView from '../views/HomeView.vue';
import ChatWidget from '../components/chat/ChatWidget.vue';
import * as chatApi from '../services/chatApi';

function makeRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/', component: HomeView },
      { path: '/dev', component: DevCatalog },
    ],
  });
}

async function openWidget(wrapper: ReturnType<typeof mount>): Promise<void> {
  await wrapper.get('.chat-widget__toggle').trigger('click');
}

async function askQuestion(
  wrapper: ReturnType<typeof mount>,
  question: string,
): Promise<void> {
  await wrapper.get('input').setValue(question);
  await wrapper.get('form').trigger('submit');
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

  it('the chat widget has zero axe violations while closed, showing only the toggle button', async () => {
    const wrapper = mount(ChatWidget, { attachTo: document.body });

    const results = await runAxe(wrapper.element);

    expect(results.violations).toEqual([]);

    wrapper.unmount();
  });

  it('the chat widget has zero axe violations with an answered message and expanded citations', async () => {
    vi.spyOn(chatApi, 'askChat').mockResolvedValue({
      answer: 'Lo sportello anagrafe apre alle 9:00.',
      sources: [{ document_id: 1, source_ref: 'Orari sportello anagrafe' }],
      fell_back: false,
    });

    const wrapper = mount(ChatWidget, { attachTo: document.body });
    await openWidget(wrapper);
    await askQuestion(wrapper, "A che ore apre l'anagrafe?");
    await flushPromises();
    await wrapper.get('summary').trigger('click');

    const results = await runAxe(wrapper.element);

    expect(results.violations).toEqual([]);

    wrapper.unmount();
  });

  it('the chat widget has zero axe violations in the honest-unknown fallback state', async () => {
    vi.spyOn(chatApi, 'askChat').mockResolvedValue({
      answer: 'Non ho trovato informazioni su questo argomento.',
      sources: [],
      fell_back: true,
    });

    const wrapper = mount(ChatWidget, { attachTo: document.body });
    await openWidget(wrapper);
    await askQuestion(wrapper, 'domanda senza risposta');
    await flushPromises();

    const results = await runAxe(wrapper.element);

    expect(results.violations).toEqual([]);

    wrapper.unmount();
  });

  it('the chat widget has zero axe violations in the error state', async () => {
    vi.spyOn(chatApi, 'askChat').mockRejectedValue(
      new chatApi.ChatApiError(502, 'upstream service unavailable'),
    );

    const wrapper = mount(ChatWidget, { attachTo: document.body });
    await openWidget(wrapper);
    await askQuestion(wrapper, 'domanda');
    await flushPromises();

    const results = await runAxe(wrapper.element);

    expect(results.violations).toEqual([]);

    wrapper.unmount();
  });

  it('the chat widget has zero axe violations in the pending state', async () => {
    vi.spyOn(chatApi, 'askChat').mockReturnValue(new Promise(() => {}));

    const wrapper = mount(ChatWidget, { attachTo: document.body });
    await openWidget(wrapper);
    await askQuestion(wrapper, 'domanda');

    const results = await runAxe(wrapper.element);

    expect(results.violations).toEqual([]);

    wrapper.unmount();
  });
});
