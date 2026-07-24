import { flushPromises, mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
import { createMemoryHistory, createRouter } from 'vue-router';

import * as adminApi from '../../services/adminApi';
import TrainingSessionView from '../TrainingSessionView.vue';

function makeRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      {
        path: '/training/:id',
        name: 'training-session',
        component: TrainingSessionView,
      },
    ],
  });
}

async function mountAtSession(id: string) {
  const router = makeRouter();
  await router.push(`/training/${id}`);
  await router.isReady();

  return mount(TrainingSessionView, { global: { plugins: [router] } });
}

const session: adminApi.TrainingSessionResponse = {
  id: 1,
  title: "Sessione sull'anagrafe",
  created_at: '2026-07-24T00:00:00Z',
  created_by: 'operator1',
  closed_at: null,
};

describe('TrainingSessionView', () => {
  it('renders a loading state and then the session with the ask/answer box', async () => {
    const getSessionSpy = vi
      .spyOn(adminApi, 'getSession')
      .mockResolvedValue(session);
    vi.spyOn(adminApi, 'listTrainingMessages').mockResolvedValue([]);

    const wrapper = await mountAtSession('1');

    expect(wrapper.text()).toContain('Caricamento della sessione');

    await flushPromises();

    expect(getSessionSpy).toHaveBeenCalledWith(1);
    expect(wrapper.text()).toContain("Sessione sull'anagrafe");
    expect(wrapper.find('form').exists()).toBe(true);
  });

  it('does not render the ask/answer box for a closed session', async () => {
    vi.spyOn(adminApi, 'getSession').mockResolvedValue({
      ...session,
      closed_at: '2026-07-25T00:00:00Z',
    });
    vi.spyOn(adminApi, 'listTrainingMessages').mockResolvedValue([]);

    const wrapper = await mountAtSession('1');
    await flushPromises();

    expect(wrapper.find('form').exists()).toBe(false);
  });

  it('renders the closed-at date for a closed session', async () => {
    vi.spyOn(adminApi, 'getSession').mockResolvedValue({
      ...session,
      closed_at: '2026-07-25T00:00:00Z',
    });
    vi.spyOn(adminApi, 'listTrainingMessages').mockResolvedValue([]);

    const wrapper = await mountAtSession('1');
    await flushPromises();

    expect(wrapper.text()).toContain('Sessione chiusa il 2026-07-25T00:00:00Z');
  });

  it('prepends a newly asked message to the message list without refetching', async () => {
    vi.spyOn(adminApi, 'getSession').mockResolvedValue(session);
    const listTrainingMessagesSpy = vi
      .spyOn(adminApi, 'listTrainingMessages')
      .mockResolvedValue([]);
    vi.spyOn(adminApi, 'askTrainingMessage').mockResolvedValue({
      id: 1,
      session_id: 1,
      question: "A che ora apre l'anagrafe?",
      answer: 'Lo sportello apre alle 9:00.',
      sources: [{ document_id: 7, source_ref: 'orari.md' }],
      fell_back: false,
      created_at: '2026-07-24T00:00:00Z',
    });

    const wrapper = await mountAtSession('1');
    await flushPromises();

    await wrapper
      .find('input[type="text"]')
      .setValue("A che ora apre l'anagrafe?");
    await wrapper.find('form').trigger('submit');
    await flushPromises();

    expect(wrapper.text()).toContain("A che ora apre l'anagrafe?");
    expect(wrapper.text()).toContain('Lo sportello apre alle 9:00.');
    expect(listTrainingMessagesSpy).toHaveBeenCalledTimes(1);
  });

  it('renders an honest not-found state for an unknown session id', async () => {
    vi.spyOn(adminApi, 'getSession').mockRejectedValue(
      new adminApi.AdminApiError(404, 'training session 999 not found'),
    );
    vi.spyOn(adminApi, 'listTrainingMessages').mockResolvedValue([]);

    const wrapper = await mountAtSession('999');
    await flushPromises();

    expect(wrapper.text()).toContain('training session 999 not found');
  });
});
