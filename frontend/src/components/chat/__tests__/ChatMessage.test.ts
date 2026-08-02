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

  it('renders the honest fallback message in a calm callout instead of citations, with no alarming title', () => {
    const wrapper = mount(ChatMessage, {
      props: { question: 'domanda', response: fallbackResponse },
    });

    expect(wrapper.findAll('li')).toHaveLength(0);
    const callout = wrapper.get('.callout-primary');
    expect(callout.text()).toBe(
      'Non ho trovato informazioni su questo argomento.',
    );
    expect(wrapper.find('.callout-title').exists()).toBe(false);
  });

  it('announces the honest fallback message to screen readers without moving focus', () => {
    const wrapper = mount(ChatMessage, {
      props: { question: 'domanda', response: fallbackResponse },
    });

    expect(wrapper.get('[role="status"]').classes()).toContain(
      'callout-primary',
    );
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

  it('renders a source with a source_url as a clickable link, labeled by the readable title', () => {
    const wrapper = mount(ChatMessage, {
      props: {
        question: 'domanda',
        response: {
          answer: 'risposta',
          sources: [
            {
              document_id: 1,
              source_ref: 'Delibera di Giunta n. 74 del 13/07/2026',
              source_url: 'https://www.halleyweb.com/detail/74',
            },
          ],
          fell_back: false,
        },
      },
    });

    const link = wrapper.find('a');
    expect(link.exists()).toBe(true);
    expect(link.attributes('href')).toBe('https://www.halleyweb.com/detail/74');
    expect(link.attributes('target')).toBe('_blank');
    expect(link.attributes('rel')).toBe('noopener noreferrer');
    expect(link.text()).toBe('Delibera di Giunta n. 74 del 13/07/2026');
  });

  it('renders a source with no source_url as plain text, not a link', () => {
    const wrapper = mount(ChatMessage, {
      props: { question: 'domanda', response: answeredResponse },
    });

    expect(wrapper.find('a').exists()).toBe(false);
  });
});
