import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';

import ChatMessage from '../ChatMessage.vue';
import type { ChatResponse } from '../../../services/chatApi';

const answeredResponse: ChatResponse = {
  answer: 'Lo sportello anagrafe apre alle 9:00.',
  sources: [
    { document_id: 1, source_ref: 'Orari sportello anagrafe' },
    { document_id: 2, source_ref: 'Regolamento comunale' },
  ],
  fell_back: false,
};

const fallbackResponse: ChatResponse = {
  answer: 'Non ho trovato informazioni su questo argomento.',
  sources: [],
  fell_back: true,
};

describe('ChatMessage', () => {
  it('renders the question and the answer text', () => {
    const wrapper = mount(ChatMessage, {
      props: {
        question: "A che ore apre l'anagrafe?",
        response: answeredResponse,
      },
    });

    expect(wrapper.text()).toContain("A che ore apre l'anagrafe?");
    expect(wrapper.text()).toContain('Lo sportello anagrafe apre alle 9:00.');
  });

  it('renders one citation per source, labeled by source_ref, when not fallen back', () => {
    const wrapper = mount(ChatMessage, {
      props: { question: 'domanda', response: answeredResponse },
    });

    const items = wrapper.findAll('li');
    expect(items).toHaveLength(2);
    expect(items[0]?.text()).toContain('Orari sportello anagrafe');
    expect(items[1]?.text()).toContain('Regolamento comunale');
  });

  it('renders a warning callout instead of citations when fell_back is true', () => {
    const wrapper = mount(ChatMessage, {
      props: { question: 'domanda', response: fallbackResponse },
    });

    expect(wrapper.findAll('li')).toHaveLength(0);
    const callout = wrapper.get(
      '[role="note"], [role="alert"], [role="status"]',
    );
    expect(callout.text()).toContain('Non ho trovato informazioni');
  });

  it('does not render a citation list when there are no sources even without a fallback', () => {
    const wrapper = mount(ChatMessage, {
      props: {
        question: 'domanda',
        response: { answer: 'risposta', sources: [], fell_back: false },
      },
    });

    expect(wrapper.findAll('li')).toHaveLength(0);
  });
});
