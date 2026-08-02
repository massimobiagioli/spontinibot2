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

async function mountAtSession() {
  const router = makeRouter();
  await router.push('/training/1');
  await router.isReady();
  return mount(TrainingSessionView, { global: { plugins: [router] } });
}

describe('TrainingSessionView integration scenario', () => {
  it('asks a question, opens its detail card, sees cited sources, and leaves negative feedback with a required reason', async () => {
    const session: adminApi.TrainingSessionResponse = {
      id: 1,
      title: "Sessione sull'anagrafe",
      created_at: '2026-07-24T00:00:00Z',
      created_by: 'operator1',
      closed_at: null,
      notes: null,
    };

    vi.spyOn(adminApi, 'getSession').mockResolvedValue(session);
    vi.spyOn(adminApi, 'listTrainingMessages').mockResolvedValue([]);
    vi.spyOn(adminApi, 'listTrainingFeedback').mockResolvedValue([]);

    const askedMessage: adminApi.TrainingMessageResponse = {
      id: 1,
      session_id: 1,
      question: "A che ora apre l'anagrafe?",
      answer: 'Lo sportello apre alle 9:00.',
      sources: [{ document_id: 7, source_ref: 'orari.md' }],
      fell_back: false,
      created_at: '2026-07-24T00:00:00Z',
      expected_answer: null,
      execution_time_ms: 84,
      source: 'chat',
    };
    vi.spyOn(adminApi, 'askTrainingMessage').mockResolvedValue(askedMessage);

    const wrapper = await mountAtSession();
    await flushPromises();

    // When the operator asks a question
    await wrapper
      .find('input[type="text"]')
      .setValue("A che ora apre l'anagrafe?");
    await wrapper.find('form').trigger('submit');
    await flushPromises();

    // Then the question appears as a card in the grid
    expect(wrapper.text()).toContain("A che ora apre l'anagrafe?");

    // When the operator opens the question's detail card
    await wrapper.find('.question-grid__card').trigger('click');
    await flushPromises();

    // Then the full answer and cited sources are shown
    expect(wrapper.text()).toContain('Lo sportello apre alle 9:00.');
    expect(wrapper.find('details summary').text()).toContain('Fonti (1)');
    expect(wrapper.text()).toContain('orari.md');
    expect(wrapper.text()).toContain('84 ms');

    // Clicking "Feedback negativo" reveals a required reason field, not a
    // span-selection UI
    const negativeButton = wrapper
      .findAll('button')
      .find((b) => b.text() === 'Feedback negativo');
    if (!negativeButton) throw new Error('negative button not found');
    await negativeButton.trigger('click');

    // Confirming without a reason is rejected client-side
    const confirmButton = wrapper
      .findAll('button')
      .find((b) => b.text() === 'Conferma feedback negativo');
    if (!confirmButton) throw new Error('confirm button not found');
    await confirmButton.trigger('click');
    await flushPromises();
    expect(wrapper.text()).toContain('Indica il motivo del feedback negativo.');

    const feedbackResponse: adminApi.TrainingFeedbackResponse = {
      id: 1,
      message_id: 1,
      chunk_id: null,
      answer_span: 'Lo sportello apre alle 9:00.',
      sentiment: 'negative',
      comment: 'orario da verificare',
      created_at: '2026-07-24T00:01:00Z',
    };
    const createTrainingFeedbackSpy = vi
      .spyOn(adminApi, 'createTrainingFeedback')
      .mockResolvedValue(feedbackResponse);

    // The session-notes textarea (from CloseSessionForm) is also on the
    // page, so target the feedback reason box by its id specifically.
    await wrapper.find('#reason-1').setValue('orario da verificare');
    await confirmButton.trigger('click');
    await flushPromises();

    expect(createTrainingFeedbackSpy).toHaveBeenCalledWith({
      message_id: 1,
      answer_span: 'Lo sportello apre alle 9:00.',
      sentiment: 'negative',
      comment: 'orario da verificare',
    });
    expect(wrapper.text()).toContain('Negativo');
    expect(wrapper.text()).toContain('orario da verificare');
  });

  it('adds a manual question and answer without invoking the bot', async () => {
    const session: adminApi.TrainingSessionResponse = {
      id: 1,
      title: 'Sessione manuale',
      created_at: '2026-07-24T00:00:00Z',
      created_by: 'operator1',
      closed_at: null,
      notes: null,
    };
    vi.spyOn(adminApi, 'getSession').mockResolvedValue(session);
    vi.spyOn(adminApi, 'listTrainingMessages').mockResolvedValue([]);
    vi.spyOn(adminApi, 'listTrainingFeedback').mockResolvedValue([]);

    const askTrainingMessageSpy = vi
      .spyOn(adminApi, 'askTrainingMessage')
      .mockResolvedValue({
        id: 2,
        session_id: 1,
        question: "Quando e' nato Gaspare Spontini?",
        answer: 'Il 14 novembre 1774',
        sources: [],
        fell_back: false,
        created_at: '2026-07-24T00:00:00Z',
        expected_answer: '1774',
        execution_time_ms: null,
        source: 'manual',
      });

    const wrapper = await mountAtSession();
    await flushPromises();

    await wrapper
      .find('input[type="text"]')
      .setValue("Quando e' nato Gaspare Spontini?");
    await wrapper.findAll('input[type="text"]')[1]!.setValue('1774');
    await wrapper.find('input[type="checkbox"]').setValue(true);
    await wrapper.find('textarea').setValue('Il 14 novembre 1774');
    await wrapper.find('form').trigger('submit');
    await flushPromises();

    expect(askTrainingMessageSpy).toHaveBeenCalledWith(1, {
      question: "Quando e' nato Gaspare Spontini?",
      expected_answer: '1774',
      answer: 'Il 14 novembre 1774',
    });
    expect(wrapper.text()).toContain("Quando e' nato Gaspare Spontini?");
  });

  it('terminates the session with closing notes', async () => {
    const session: adminApi.TrainingSessionResponse = {
      id: 1,
      title: 'Sessione da chiudere',
      created_at: '2026-07-24T00:00:00Z',
      created_by: 'operator1',
      closed_at: null,
      notes: null,
    };
    vi.spyOn(adminApi, 'getSession')
      .mockResolvedValueOnce(session)
      .mockResolvedValueOnce({
        ...session,
        closed_at: '2026-07-24T01:00:00Z',
        notes: 'Sessione conclusa senza problemi',
      });
    vi.spyOn(adminApi, 'listTrainingMessages').mockResolvedValue([]);
    const closeSessionSpy = vi
      .spyOn(adminApi, 'closeSession')
      .mockResolvedValue({ closed: true });

    const wrapper = await mountAtSession();
    await flushPromises();

    await wrapper.find('textarea').setValue('Sessione conclusa senza problemi');
    await wrapper.find('button.btn-outline-danger').trigger('click');

    const dialog = wrapper.find('[data-testid="close-session-dialog"]');
    expect(dialog.attributes('open')).toBeDefined();
    await dialog.find('button.btn-danger').trigger('click');
    await flushPromises();

    expect(closeSessionSpy).toHaveBeenCalledWith(
      1,
      'Sessione conclusa senza problemi',
    );
    expect(wrapper.text()).toContain('Sessione conclusa senza problemi');
    expect(wrapper.text()).not.toContain('Aggiungi domanda');
  });
});
