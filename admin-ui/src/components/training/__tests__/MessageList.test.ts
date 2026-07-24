import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import * as adminApi from '../../../services/adminApi';
import type { TrainingMessageResponse } from '../../../services/adminApi';
import MessageList from '../MessageList.vue';

describe('MessageList', () => {
  beforeEach(() => {
    vi.spyOn(adminApi, 'listTrainingFeedback').mockResolvedValue([]);
  });

  it('renders the answer with a collapsible list of cited sources', async () => {
    const messages: TrainingMessageResponse[] = [
      {
        id: 1,
        session_id: 1,
        question: "A che ora apre l'anagrafe?",
        answer: 'Lo sportello apre alle 9:00.',
        sources: [
          { document_id: 7, source_ref: 'orari.md' },
          { document_id: 8, source_ref: 'regolamento.pdf' },
        ],
        fell_back: false,
        created_at: '2026-07-24T00:00:00Z',
      },
    ];

    const wrapper = mount(MessageList, { props: { messages } });
    await flushPromises();

    expect(wrapper.text()).toContain("A che ora apre l'anagrafe?");
    expect(wrapper.text()).toContain('Lo sportello apre alle 9:00.');
    expect(wrapper.find('details summary').text()).toContain('Fonti (2)');
    expect(wrapper.text()).toContain('orari.md');
    expect(wrapper.text()).toContain('regolamento.pdf');
  });

  it('renders the honest-unknown callout with zero citations when fell_back is true', async () => {
    const messages: TrainingMessageResponse[] = [
      {
        id: 2,
        session_id: 1,
        question: 'Qual è la popolazione della luna?',
        answer: 'Non ho trovato informazioni nei documenti comunali.',
        sources: [],
        fell_back: true,
        created_at: '2026-07-24T00:00:00Z',
      },
    ];

    const wrapper = mount(MessageList, { props: { messages } });
    await flushPromises();

    expect(wrapper.text()).toContain('Nessuna informazione trovata');
    expect(wrapper.find('details').exists()).toBe(false);
  });

  it('renders nothing when there are no messages', () => {
    const wrapper = mount(MessageList, { props: { messages: [] } });

    expect(wrapper.findAll('article').length).toBe(0);
  });
});
