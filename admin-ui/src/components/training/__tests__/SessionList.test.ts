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
  notes: null,
};

const closedSession: adminApi.TrainingSessionResponse = {
  id: 2,
  title: 'Sessione chiusa',
  created_at: '2026-07-20T00:00:00Z',
  created_by: 'operator1',
  closed_at: '2026-07-21T00:00:00Z',
  notes: 'tutto ok',
};

async function mountWithRouter(sessions: adminApi.TrainingSessionResponse[]) {
  const router = makeRouter();
  await router.push('/training');
  await router.isReady();

  const wrapper = mount(SessionList, {
    props: { sessions },
    global: { plugins: [router] },
  });
  return { wrapper, router };
}

describe('SessionList', () => {
  it('renders open sessions with an "Aperta" badge and closed sessions with a "Chiusa" badge', async () => {
    const { wrapper } = await mountWithRouter([openSession, closedSession]);

    expect(wrapper.text()).toContain('Sessione aperta');
    expect(wrapper.text()).toContain('Sessione chiusa');
    expect(wrapper.text()).toContain('Aperta');
    expect(wrapper.text()).toContain('Chiusa');
    // Every session (open or closed) offers a delete button.
    expect(wrapper.findAll('li button').length).toBe(2);
  });

  it('links each session to its detail route', async () => {
    const { wrapper } = await mountWithRouter([openSession]);

    const link = wrapper.get('a');
    expect(link.attributes('href')).toBe('/training/1');
  });

  it('is a clickable card, like the question cards in a session: clicking anywhere on it navigates to the session', async () => {
    const { wrapper, router } = await mountWithRouter([openSession]);

    expect(wrapper.get('.session-list__card').classes()).toContain(
      'clickable-card',
    );

    await wrapper.get('.session-list__card').trigger('click');
    await flushPromises();

    expect(router.currentRoute.value.path).toBe('/training/1');
  });

  it('does not navigate when the delete button inside the card is clicked', async () => {
    const { wrapper, router } = await mountWithRouter([openSession]);

    await wrapper.find('li button').trigger('click');

    expect(router.currentRoute.value.path).toBe('/training');
  });

  it('calls createSession when the add-session form is submitted', async () => {
    const createSessionSpy = vi
      .spyOn(adminApi, 'createSession')
      .mockResolvedValue(openSession);

    const { wrapper } = await mountWithRouter([]);

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

    const { wrapper } = await mountWithRouter([]);

    await wrapper.find('input[type="text"]').setValue('');
    await wrapper.find('form').trigger('submit');
    await flushPromises();

    expect(wrapper.text()).toContain('invalid title');
    expect(wrapper.emitted('changed')).toBeFalsy();
  });

  it('opens the confirm dialog before calling deleteSession, and refreshes on confirm', async () => {
    const deleteSessionSpy = vi
      .spyOn(adminApi, 'deleteSession')
      .mockResolvedValue({ deleted: true });

    const { wrapper } = await mountWithRouter([openSession]);

    await wrapper.find('li button').trigger('click');
    expect(deleteSessionSpy).not.toHaveBeenCalled();

    const dialog = wrapper.find('[data-testid="delete-session-dialog"]');
    expect(dialog.attributes('open')).toBeDefined();

    await dialog.find('button.btn-danger').trigger('click');
    await flushPromises();

    expect(deleteSessionSpy).toHaveBeenCalledWith(1);
    expect(wrapper.emitted('changed')).toBeTruthy();
  });

  it('does not call deleteSession when the dialog is cancelled', async () => {
    const deleteSessionSpy = vi.spyOn(adminApi, 'deleteSession');

    const { wrapper } = await mountWithRouter([openSession]);

    await wrapper.find('li button').trigger('click');
    await wrapper
      .find('[data-testid="delete-session-dialog"]')
      .find('button.btn-outline-secondary')
      .trigger('click');

    expect(deleteSessionSpy).not.toHaveBeenCalled();
  });

  it('shows the honest error message from AdminApiError on delete failure', async () => {
    vi.spyOn(adminApi, 'deleteSession').mockRejectedValue(
      new adminApi.AdminApiError(500, 'internal error'),
    );

    const { wrapper } = await mountWithRouter([openSession]);

    await wrapper.find('li button').trigger('click');
    await wrapper
      .find('[data-testid="delete-session-dialog"]')
      .find('button.btn-danger')
      .trigger('click');
    await flushPromises();

    expect(wrapper.text()).toContain('internal error');
  });

  function makeSessions(count: number): adminApi.TrainingSessionResponse[] {
    return Array.from({ length: count }, (_, i) => ({
      id: i + 1,
      title: `Sessione ${i + 1}`,
      created_at: '2026-07-24T00:00:00Z',
      created_by: 'operator1',
      closed_at: null,
      notes: null,
    }));
  }

  it('does not show pagination when everything fits on one page', async () => {
    const { wrapper } = await mountWithRouter(makeSessions(9));

    expect(
      wrapper.find('nav[aria-label="Paginazione sessioni"]').exists(),
    ).toBe(false);
  });

  it('paginates in blocks of 9, with page-number controls once there is a second page', async () => {
    const { wrapper } = await mountWithRouter(makeSessions(10));

    const cardTitle = () =>
      wrapper.findAll('.it-card-title').map((el) => el.text());

    expect(cardTitle()).toHaveLength(9);
    expect(cardTitle()).toContain('Sessione 1');
    expect(cardTitle()).not.toContain('Sessione 10');

    const nav = wrapper.get('nav[aria-label="Paginazione sessioni"]');
    const pageButtons = nav.findAll('li.page-item button');
    // prev + page 1 + page 2 + next
    expect(pageButtons.length).toBe(4);

    await pageButtons[2]!.trigger('click');

    expect(cardTitle()).toHaveLength(1);
    expect(cardTitle()).toContain('Sessione 10');
  });
});
