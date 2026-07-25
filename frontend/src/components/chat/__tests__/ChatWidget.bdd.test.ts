import { flushPromises, mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';

import * as chatApi from '../../../services/chatApi';
import ChatWidget from '../ChatWidget.vue';

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
});
