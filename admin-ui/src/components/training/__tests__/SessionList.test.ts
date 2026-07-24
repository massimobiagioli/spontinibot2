import { flushPromises, mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
import { createMemoryHistory, createRouter } from 'vue-router';

import * as adminApi from '../../../services/adminApi';
import SessionList from '../SessionList.vue';

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

const openSession: adminApi.TrainingSessionResponse = {
  id: 1,
  title: 'Sessione aperta',
  created_at: '2026-07-24T00:00:00Z',
  created_by: 'operator1',
  closed_at: null,
};

const closedSession: adminApi.TrainingSessionResponse = {
  id: 2,
  title: 'Sessione chiusa',
  created_at: '2026-07-20T00:00:00Z',
  created_by: 'operator1',
  closed_at: '2026-07-21T00:00:00Z',
};

async function mountWithRouter(sessions: adminApi.TrainingSessionResponse[]) {
  const router = makeRouter();
  await router.push('/training');
  await router.isReady();

  return mount(SessionList, {
    props: { sessions },
    global: { plugins: [router] },
  });
}

describe('SessionList', () => {
  it('renders open sessions with a close button and closed sessions with a badge', async () => {
    const wrapper = await mountWithRouter([openSession, closedSession]);

    expect(wrapper.text()).toContain('Sessione aperta');
    expect(wrapper.text()).toContain('Sessione chiusa');
    expect(wrapper.text()).toContain('Chiusa');
    expect(wrapper.findAll('li button').length).toBe(1);
  });

  it('links each session to its detail route', async () => {
    const wrapper = await mountWithRouter([openSession]);

    const link = wrapper.get('a');
    expect(link.attributes('href')).toBe('/training/1');
  });

  it('calls createSession when the add-session form is submitted', async () => {
    const createSessionSpy = vi
      .spyOn(adminApi, 'createSession')
      .mockResolvedValue(openSession);

    const wrapper = await mountWithRouter([]);

    await wrapper.find('input[type="text"]').setValue('Sessione di prova');
    await wrapper.find('form').trigger('submit');
    await flushPromises();

    expect(createSessionSpy).toHaveBeenCalledWith('Sessione di prova');
    expect(wrapper.emitted('changed')).toBeTruthy();
  });

  it('shows the honest error message from AdminApiError on create failure', async () => {
    vi.spyOn(adminApi, 'createSession').mockRejectedValue(
      new adminApi.AdminApiError(400, 'invalid title'),
    );

    const wrapper = await mountWithRouter([]);

    await wrapper.find('input[type="text"]').setValue('');
    await wrapper.find('form').trigger('submit');
    await flushPromises();

    expect(wrapper.text()).toContain('invalid title');
    expect(wrapper.emitted('changed')).toBeFalsy();
  });

  it('opens the confirm dialog before calling closeSession, and refreshes on confirm', async () => {
    const closeSessionSpy = vi
      .spyOn(adminApi, 'closeSession')
      .mockResolvedValue({ closed: true });

    const wrapper = await mountWithRouter([openSession]);

    await wrapper.find('li button').trigger('click');
    expect(closeSessionSpy).not.toHaveBeenCalled();

    const dialog = wrapper.find('[data-testid="close-session-dialog"]');
    expect(dialog.attributes('open')).toBeDefined();

    await dialog.find('button.btn-danger').trigger('click');
    await flushPromises();

    expect(closeSessionSpy).toHaveBeenCalledWith(1);
    expect(wrapper.emitted('changed')).toBeTruthy();
  });

  it('does not call closeSession when the dialog is cancelled', async () => {
    const closeSessionSpy = vi.spyOn(adminApi, 'closeSession');

    const wrapper = await mountWithRouter([openSession]);

    await wrapper.find('li button').trigger('click');
    await wrapper
      .find('[data-testid="close-session-dialog"]')
      .find('button.btn-outline-secondary')
      .trigger('click');

    expect(closeSessionSpy).not.toHaveBeenCalled();
  });

  it('shows the honest error message from AdminApiError on close failure', async () => {
    vi.spyOn(adminApi, 'closeSession').mockRejectedValue(
      new adminApi.AdminApiError(500, 'internal error'),
    );

    const wrapper = await mountWithRouter([openSession]);

    await wrapper.find('li button').trigger('click');
    await wrapper
      .find('[data-testid="close-session-dialog"]')
      .find('button.btn-danger')
      .trigger('click');
    await flushPromises();

    expect(wrapper.text()).toContain('internal error');
  });
});
