import { flushPromises, mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
import { createMemoryHistory, createRouter } from 'vue-router';

import * as adminApi from '../../services/adminApi';
import TrainingSessionsView from '../TrainingSessionsView.vue';

function makeRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      {
        path: '/training',
        name: 'training',
        component: { template: '<div />' },
      },
      {
        path: '/training/:id',
        name: 'training-session',
        component: { template: '<div />' },
      },
    ],
  });
}

async function mountWithRouter() {
  const router = makeRouter();
  await router.push('/training');
  await router.isReady();

  return mount(TrainingSessionsView, { global: { plugins: [router] } });
}

describe('TrainingSessionsView', () => {
  it('renders a loading state and then the resolved session count', async () => {
    const listSessionsSpy = vi
      .spyOn(adminApi, 'listSessions')
      .mockResolvedValue([
        {
          id: 1,
          title: 'Sessione',
          created_at: 'now',
          created_by: null,
          closed_at: null,
          notes: null,
        },
      ]);

    const wrapper = await mountWithRouter();

    expect(wrapper.text()).toContain('Caricamento delle sessioni');

    await flushPromises();

    expect(listSessionsSpy).toHaveBeenCalledOnce();
    expect(wrapper.text()).not.toContain('Caricamento delle sessioni');
    expect(wrapper.text()).toContain('1 sessioni totali');
  });

  it('renders an honest empty state when no session exists yet', async () => {
    vi.spyOn(adminApi, 'listSessions').mockResolvedValue([]);

    const wrapper = await mountWithRouter();
    await flushPromises();

    expect(wrapper.text()).toContain('Nessuna sessione');
  });

  it('renders an honest error state when the fetch fails', async () => {
    vi.spyOn(adminApi, 'listSessions').mockRejectedValue(
      new adminApi.AdminApiError(401, 'invalid or missing session cookie'),
    );

    const wrapper = await mountWithRouter();
    await flushPromises();

    expect(wrapper.text()).toContain('invalid or missing session cookie');
  });
});
