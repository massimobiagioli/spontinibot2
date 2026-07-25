import { flushPromises, mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';

import * as chatApi from '../../../services/chatApi';
import ChatWidget from '../ChatWidget.vue';

async function openWidget(wrapper: ReturnType<typeof mount>): Promise<void> {
  await wrapper.get('.chat-widget__toggle').trigger('click');
}

describe('Feature: citizen asks a question and expands the cited sources', () => {
  it('Scenario: a citizen asks a question answerable from a municipal document', async () => {
    // Given the knowledge base contains a document titled "Orari sportello anagrafe"
    // and Spontini can answer the citizen's question by citing it
    vi.spyOn(chatApi, 'askChat').mockResolvedValue({
      answer: "Lo sportello anagrafe e' aperto dal lunedi' al venerdi'.",
      sources: [{ document_id: 1, source_ref: 'Orari sportello anagrafe' }],
      fell_back: false,
    });
    const wrapper = mount(ChatWidget);

    // And the citizen opens the chat widget
    await openWidget(wrapper);

    // When the citizen asks "A che ore apre l'anagrafe?"
    await wrapper.get('input').setValue("A che ore apre l'anagrafe?");
    await wrapper.get('form').trigger('submit');
    await flushPromises();

    // Then Spontini answers using the content of the retrieved document
    expect(wrapper.text()).toContain(
      "Lo sportello anagrafe e' aperto dal lunedi' al venerdi'.",
    );

    // And the citizen can expand the citation list to see the cited source
    const disclosure = wrapper.get('details');
    expect(disclosure.attributes('open')).toBeUndefined();
    expect(disclosure.text()).toContain('Orari sportello anagrafe');

    await disclosure.get('summary').trigger('click');

    expect((disclosure.element as HTMLDetailsElement).open).toBe(true);
  });

  it('Scenario: a citizen asks a question not answerable from any document', async () => {
    // Given the knowledge base contains no document about "tasse comunali"
    // and Spontini honestly says so instead of inventing an answer
    vi.spyOn(chatApi, 'askChat').mockResolvedValue({
      answer: 'Non ho trovato informazioni su questo argomento.',
      sources: [],
      fell_back: true,
    });
    const wrapper = mount(ChatWidget);
    await openWidget(wrapper);

    // When the citizen asks "Quanto pago di tasse comunali?"
    await wrapper.get('input').setValue('Quanto pago di tasse comunali?');
    await wrapper.get('form').trigger('submit');
    await flushPromises();

    // Then Spontini answers that no information was found, calmly, with no citations
    expect(wrapper.text()).toContain(
      'Non ho trovato informazioni su questo argomento.',
    );
    expect(wrapper.findAll('details')).toHaveLength(0);

    // And Spontini does not invent any detail — the fallback text is the only content shown
    expect(wrapper.text()).not.toContain('tasse comunali sono');
  });

  it('Scenario: a citizen asks a question while the backend cannot answer', async () => {
    // Given /chat is returning an upstream error (e.g. 502/503)
    vi.spyOn(chatApi, 'askChat').mockRejectedValue(
      new chatApi.ChatApiError(503, 'no active persona configured'),
    );
    const wrapper = mount(ChatWidget);
    await openWidget(wrapper);

    // When the citizen asks a question
    await wrapper.get('input').setValue('A che ore apre l-anagrafe?');
    await wrapper.get('form').trigger('submit');
    await flushPromises();

    // Then Spontini honestly says it cannot answer right now
    expect(wrapper.text()).toContain('Non riesco a rispondere ora');

    // And the raw backend error is never shown to the citizen
    expect(wrapper.text()).not.toContain('no active persona configured');
  });
});
