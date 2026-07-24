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

describe('TrainingSessionView integration scenario', () => {
  it('ask a question, see cited sources, leave negative feedback on a span, see the feedback persisted', async () => {
    const session: adminApi.TrainingSessionResponse = {
      id: 1,
      title: "Sessione sull'anagrafe",
      created_at: '2026-07-24T00:00:00Z',
      created_by: 'operator1',
      closed_at: null,
    };

    // Given an existing open session with no messages
    vi.spyOn(adminApi, 'getSession').mockResolvedValue(session);
    vi.spyOn(adminApi, 'listTrainingMessages').mockResolvedValue([]);
    vi.spyOn(adminApi, 'listTrainingFeedback').mockResolvedValue([]);

    const askedMessage: adminApi.TrainingMessageResponse = {
      id: 1,
      session_id: 1,
      question: "A che ora apre l'anagrafe?",
      answer: 'Lo sportello apre alle 9:00. Il sabato è chiuso.',
      sources: [{ document_id: 7, source_ref: 'orari.md' }],
      fell_back: false,
      created_at: '2026-07-24T00:00:00Z',
    };
    vi.spyOn(adminApi, 'askTrainingMessage').mockResolvedValue(askedMessage);

    const router = makeRouter();
    await router.push('/training/1');
    await router.isReady();
    const wrapper = mount(TrainingSessionView, {
      global: { plugins: [router] },
    });
    await flushPromises();

    // When the operator asks a question
    await wrapper
      .find('input[type="text"]')
      .setValue("A che ora apre l'anagrafe?");
    await wrapper.find('form').trigger('submit');
    await flushPromises();

    // Then the answer renders with its cited sources
    expect(wrapper.text()).toContain('Lo sportello apre alle 9:00.');
    expect(wrapper.find('details summary').text()).toContain('Fonti (1)');
    expect(wrapper.text()).toContain('orari.md');

    // When the operator clicks a sentence segment of the answer and submits
    // negative feedback with a comment
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

    const segment = wrapper.find('.span-feedback__segment');
    await segment.trigger('click');
    await wrapper.find('textarea').setValue('orario da verificare');
    const negativeButton = wrapper
      .findAll('button')
      .find((b) => b.text() === 'Feedback negativo');
    if (!negativeButton) throw new Error('negative button not found');
    await negativeButton.trigger('click');
    await flushPromises();

    // Then the feedback appears in the persisted feedback list for that message
    expect(createTrainingFeedbackSpy).toHaveBeenCalledWith({
      message_id: 1,
      answer_span: 'Lo sportello apre alle 9:00.',
      sentiment: 'negative',
      comment: 'orario da verificare',
    });
    expect(wrapper.text()).toContain('Negativo');
    expect(wrapper.text()).toContain('orario da verificare');
  });
});
