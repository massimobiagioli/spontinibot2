import { flushPromises, mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';

import * as chatApi from '../../../services/chatApi';
import ChatWidget from '../ChatWidget.vue';

const answeredResponse: chatApi.ChatResponse = {
  answer: 'Lo sportello anagrafe apre alle 9:00.',
  sources: [{ document_id: 1, source_ref: 'Orari sportello anagrafe' }],
  fell_back: false,
};

async function askQuestion(
  wrapper: ReturnType<typeof mount>,
  question: string,
): Promise<void> {
  await wrapper.get('input').setValue(question);
  await wrapper.get('form').trigger('submit');
}

describe('ChatWidget', () => {
  it('shows an empty-conversation state before any question is asked', () => {
    const wrapper = mount(ChatWidget);

    expect(wrapper.text()).toContain('Fai una domanda');
    expect(wrapper.findAll('article')).toHaveLength(0);
  });

  it('appends a ChatMessage with the answer after asking a question', async () => {
    vi.spyOn(chatApi, 'askChat').mockResolvedValue(answeredResponse);
    const wrapper = mount(ChatWidget);

    await askQuestion(wrapper, "A che ore apre l'anagrafe?");
    await flushPromises();

    expect(wrapper.text()).toContain("A che ore apre l'anagrafe?");
    expect(wrapper.text()).toContain('Lo sportello anagrafe apre alle 9:00.');
    expect(wrapper.findAll('article')).toHaveLength(1);
  });

  it('disables the input while a request is in flight', async () => {
    let resolveAsk!: (value: chatApi.ChatResponse) => void;
    vi.spyOn(chatApi, 'askChat').mockReturnValue(
      new Promise((resolve) => {
        resolveAsk = resolve;
      }),
    );
    const wrapper = mount(ChatWidget);

    await askQuestion(wrapper, 'domanda');

    expect(wrapper.get('input').attributes('disabled')).toBeDefined();

    resolveAsk(answeredResponse);
    await flushPromises();

    expect(wrapper.get('input').attributes('disabled')).toBeUndefined();
  });

  it('appends an error callout instead of crashing when askChat rejects', async () => {
    vi.spyOn(chatApi, 'askChat').mockRejectedValue(
      new chatApi.ChatApiError(502, 'upstream service unavailable'),
    );
    const wrapper = mount(ChatWidget);

    await askQuestion(wrapper, 'domanda');
    await flushPromises();

    expect(wrapper.text()).toContain('upstream service unavailable');
    expect(wrapper.get('[role="alert"]')).toBeTruthy();
  });
});
