import { flushPromises, mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';

import * as adminApi from '../../../services/adminApi';
import AskAnswerBox from '../AskAnswerBox.vue';

const message: adminApi.TrainingMessageResponse = {
  id: 1,
  session_id: 1,
  question: "A che ora apre l'anagrafe?",
  answer: 'Lo sportello apre alle 9:00.',
  sources: [{ document_id: 7, source_ref: 'orari.md' }],
  fell_back: false,
  created_at: '2026-07-24T00:00:00Z',
};

describe('AskAnswerBox', () => {
  it('submits the question and emits asked with the new message', async () => {
    const askTrainingMessageSpy = vi
      .spyOn(adminApi, 'askTrainingMessage')
      .mockResolvedValue(message);

    const wrapper = mount(AskAnswerBox, { props: { sessionId: 1 } });

    await wrapper
      .find('input[type="text"]')
      .setValue("A che ora apre l'anagrafe?");
    await wrapper.find('form').trigger('submit');
    await flushPromises();

    expect(askTrainingMessageSpy).toHaveBeenCalledWith(
      1,
      "A che ora apre l'anagrafe?",
    );
    expect(wrapper.emitted('asked')?.[0]).toEqual([message]);
    expect(
      (wrapper.find('input[type="text"]').element as HTMLInputElement).value,
    ).toBe('');
  });

  it('shows the honest error message from AdminApiError on failure', async () => {
    vi.spyOn(adminApi, 'askTrainingMessage').mockRejectedValue(
      new adminApi.AdminApiError(502, 'generation service error: timeout'),
    );

    const wrapper = mount(AskAnswerBox, { props: { sessionId: 1 } });

    await wrapper.find('input[type="text"]').setValue('domanda');
    await wrapper.find('form').trigger('submit');
    await flushPromises();

    expect(wrapper.text()).toContain('generation service error: timeout');
    expect(wrapper.emitted('asked')).toBeFalsy();
  });
});
