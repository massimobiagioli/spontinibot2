import { flushPromises, mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';

import * as chatApi from '../../../services/chatApi';
import ChatWidget from '../ChatWidget.vue';

const answeredResponse: chatApi.ChatResponse = {
  answer: 'Lo sportello anagrafe apre alle 9:00.',
  sources: [{ document_id: 1, source_ref: 'Orari sportello anagrafe' }],
  fell_back: false,
};

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

describe('ChatWidget', () => {
  it('starts closed, showing only the toggle button', () => {
    const wrapper = mount(ChatWidget);

    expect(wrapper.find('.chat-widget__panel').exists()).toBe(false);
    expect(
      wrapper.get('.chat-widget__toggle').attributes('aria-expanded'),
    ).toBe('false');
  });

  it('shows an empty-conversation state before any question is asked', async () => {
    const wrapper = mount(ChatWidget);
    await openWidget(wrapper);

    expect(wrapper.text()).toContain('Fai una domanda');
    expect(wrapper.findAll('article')).toHaveLength(0);
  });

  it('appends a ChatMessage with the answer after asking a question', async () => {
    vi.spyOn(chatApi, 'askChat').mockResolvedValue(answeredResponse);
    const wrapper = mount(ChatWidget);
    await openWidget(wrapper);

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
    await openWidget(wrapper);

    await askQuestion(wrapper, 'domanda');

    expect(wrapper.get('input').attributes('disabled')).toBeDefined();

    resolveAsk(answeredResponse);
    await flushPromises();

    expect(wrapper.get('input').attributes('disabled')).toBeUndefined();
  });

  it('shows the fixed honest error message, never the raw backend error, when askChat rejects with a ChatApiError', async () => {
    vi.spyOn(chatApi, 'askChat').mockRejectedValue(
      new chatApi.ChatApiError(502, 'upstream service unavailable'),
    );
    const wrapper = mount(ChatWidget);
    await openWidget(wrapper);

    await askQuestion(wrapper, 'domanda');
    await flushPromises();

    expect(wrapper.text()).toContain('Non riesco a rispondere ora');
    expect(wrapper.text()).not.toContain('upstream service unavailable');
    expect(wrapper.get('[role="alert"]')).toBeTruthy();
  });

  it('shows the same fixed honest error message when askChat rejects with a non-ChatApiError failure', async () => {
    vi.spyOn(chatApi, 'askChat').mockRejectedValue(
      new TypeError('Failed to fetch'),
    );
    const wrapper = mount(ChatWidget);
    await openWidget(wrapper);

    await askQuestion(wrapper, 'domanda');
    await flushPromises();

    expect(wrapper.text()).toContain('Non riesco a rispondere ora');
    expect(wrapper.text()).not.toContain('Failed to fetch');
    expect(wrapper.get('[role="alert"]')).toBeTruthy();
  });
});
