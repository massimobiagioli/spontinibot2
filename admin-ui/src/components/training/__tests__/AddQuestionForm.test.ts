import { flushPromises, mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';

import * as adminApi from '../../../services/adminApi';
import AddQuestionForm from '../AddQuestionForm.vue';

const askedMessage: adminApi.TrainingMessageResponse = {
  id: 1,
  session_id: 1,
  question: "A che ora apre l'anagrafe?",
  answer: 'Lo sportello apre alle 9:00.',
  sources: [],
  fell_back: false,
  created_at: '2026-07-24T00:00:00Z',
  expected_answer: null,
  execution_time_ms: 84,
  source: 'chat',
};

describe('AddQuestionForm', () => {
  it('asks the bot live by default, without a manual answer', async () => {
    const askTrainingMessageSpy = vi
      .spyOn(adminApi, 'askTrainingMessage')
      .mockResolvedValue(askedMessage);

    const wrapper = mount(AddQuestionForm, { props: { sessionId: 1 } });

    await wrapper
      .find('input[type="text"]')
      .setValue("A che ora apre l'anagrafe?");
    await wrapper.find('form').trigger('submit');
    await flushPromises();

    expect(askTrainingMessageSpy).toHaveBeenCalledWith(1, {
      question: "A che ora apre l'anagrafe?",
    });
    expect(wrapper.emitted('added')?.[0]).toEqual([askedMessage]);
  });

  it('includes an optional expected answer when provided', async () => {
    const askTrainingMessageSpy = vi
      .spyOn(adminApi, 'askTrainingMessage')
      .mockResolvedValue(askedMessage);

    const wrapper = mount(AddQuestionForm, { props: { sessionId: 1 } });

    const inputs = wrapper.findAll('input[type="text"]');
    await inputs[0]!.setValue('domanda');
    await inputs[1]!.setValue('risposta attesa');
    await wrapper.find('form').trigger('submit');
    await flushPromises();

    expect(askTrainingMessageSpy).toHaveBeenCalledWith(1, {
      question: 'domanda',
      expected_answer: 'risposta attesa',
    });
  });

  it('submits a manual answer without invoking the bot when manual mode is checked', async () => {
    const askTrainingMessageSpy = vi
      .spyOn(adminApi, 'askTrainingMessage')
      .mockResolvedValue({ ...askedMessage, source: 'manual' });

    const wrapper = mount(AddQuestionForm, { props: { sessionId: 1 } });

    await wrapper.find('input[type="text"]').setValue('domanda');
    await wrapper.find('input[type="checkbox"]').setValue(true);
    await wrapper.find('textarea').setValue('risposta manuale');
    await wrapper.find('form').trigger('submit');
    await flushPromises();

    expect(askTrainingMessageSpy).toHaveBeenCalledWith(1, {
      question: 'domanda',
      answer: 'risposta manuale',
    });
  });

  it('shows the honest error message from AdminApiError on failure', async () => {
    vi.spyOn(adminApi, 'askTrainingMessage').mockRejectedValue(
      new adminApi.AdminApiError(502, 'generation service error: timeout'),
    );

    const wrapper = mount(AddQuestionForm, { props: { sessionId: 1 } });

    await wrapper.find('input[type="text"]').setValue('domanda');
    await wrapper.find('form').trigger('submit');
    await flushPromises();

    expect(wrapper.text()).toContain('generation service error: timeout');
  });
});
