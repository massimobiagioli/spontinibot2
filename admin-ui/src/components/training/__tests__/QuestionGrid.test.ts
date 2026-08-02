import { flushPromises, mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';

import * as adminApi from '../../../services/adminApi';
import QuestionGrid from '../QuestionGrid.vue';

const messages: adminApi.TrainingMessageResponse[] = [
  {
    id: 1,
    session_id: 1,
    question: "A che ora apre l'anagrafe?",
    answer: 'Lo sportello apre alle 9:00.',
    sources: [{ document_id: 7, source_ref: 'orari.md' }],
    fell_back: false,
    created_at: '2026-07-24T00:00:00Z',
    expected_answer: null,
    execution_time_ms: 84,
    source: 'chat',
  },
  {
    id: 2,
    session_id: 1,
    question: 'Domanda inserita manualmente',
    answer: 'Risposta manuale',
    sources: [],
    fell_back: false,
    created_at: '2026-07-24T00:05:00Z',
    expected_answer: 'atteso',
    execution_time_ms: null,
    source: 'manual',
  },
];

describe('QuestionGrid', () => {
  it('renders one card per question with its source tag and duration', async () => {
    vi.spyOn(adminApi, 'listTrainingFeedback').mockResolvedValue([]);

    const wrapper = mount(QuestionGrid, { props: { messages } });
    await flushPromises();

    const cards = wrapper.findAll('.question-grid__card');
    expect(cards.length).toBe(2);
    expect(wrapper.text()).toContain("A che ora apre l'anagrafe?");
    expect(wrapper.text()).toContain('84 ms');
    expect(wrapper.text()).toContain('Bot');
    expect(wrapper.text()).toContain('Manuale');
  });

  it('renders an honest empty state when the session has no questions', async () => {
    const wrapper = mount(QuestionGrid, { props: { messages: [] } });
    await flushPromises();

    expect(wrapper.text()).toContain('Nessuna domanda in questa sessione.');
  });

  it('shows the latest feedback verdict as a badge on each card', async () => {
    vi.spyOn(adminApi, 'listTrainingFeedback').mockImplementation(
      async (messageId: number) => {
        if (messageId === 1) {
          return [
            {
              id: 1,
              message_id: 1,
              chunk_id: null,
              answer_span: 'x',
              sentiment: 'positive',
              comment: null,
              created_at: '2026-07-24T00:00:00Z',
            },
          ];
        }
        return [];
      },
    );

    const wrapper = mount(QuestionGrid, { props: { messages } });
    await flushPromises();

    const cards = wrapper.findAll('.question-grid__card');
    expect(cards[0]!.text()).toContain('Positivo');
    expect(cards[1]!.text()).toContain('Nessun feedback');
  });

  it('opens the question detail dialog when a card is clicked, and closes it', async () => {
    vi.spyOn(adminApi, 'listTrainingFeedback').mockResolvedValue([]);

    const wrapper = mount(QuestionGrid, { props: { messages } });
    await flushPromises();

    expect(wrapper.text()).not.toContain('Scheda domanda');

    await wrapper.findAll('.question-grid__card')[0]!.trigger('click');
    expect(wrapper.text()).toContain('Scheda domanda');

    await wrapper.find('button.btn-outline-secondary').trigger('click');
    expect(wrapper.text()).not.toContain('Scheda domanda');
  });

  function manyMessages(count: number): adminApi.TrainingMessageResponse[] {
    return Array.from({ length: count }, (_, i) => ({
      id: i + 1,
      session_id: 1,
      question: `Domanda numero ${i + 1}`,
      answer: 'Risposta.',
      sources: [],
      fell_back: false,
      created_at: '2026-07-24T00:00:00Z',
      expected_answer: null,
      execution_time_ms: null,
      source: 'chat' as const,
    }));
  }

  it('paginates the questions 20 per page, matching a 100-question test session', async () => {
    vi.spyOn(adminApi, 'listTrainingFeedback').mockResolvedValue([]);

    const wrapper = mount(QuestionGrid, {
      props: { messages: manyMessages(100) },
    });
    await flushPromises();

    expect(wrapper.findAll('.question-grid__card')).toHaveLength(20);
    const nav = wrapper.get('nav[aria-label="Paginazione domande"]');
    expect(nav.findAll('li.page-item')).toHaveLength(7); // prev + 5 pages + next

    await wrapper.findAll('li.page-item button')[2]!.trigger('click'); // page "2"
    await flushPromises();

    expect(wrapper.findAll('.question-grid__card')).toHaveLength(20);
    expect(wrapper.text()).toContain('Domanda numero 21');
  });

  it('does not show pagination controls when there are 20 or fewer questions', async () => {
    vi.spyOn(adminApi, 'listTrainingFeedback').mockResolvedValue([]);

    const wrapper = mount(QuestionGrid, { props: { messages } });
    await flushPromises();

    expect(wrapper.find('nav[aria-label="Paginazione domande"]').exists()).toBe(
      false,
    );
  });

  it('resets to page 1 when the message set changes (e.g. switching sessions)', async () => {
    vi.spyOn(adminApi, 'listTrainingFeedback').mockResolvedValue([]);

    const wrapper = mount(QuestionGrid, {
      props: { messages: manyMessages(30) },
    });
    await flushPromises();

    const pageButtons = () =>
      wrapper
        .get('nav[aria-label="Paginazione domande"]')
        .findAll('li.page-item button');

    await pageButtons()[2]!.trigger('click'); // page "2"
    await flushPromises();
    expect(pageButtons()[2]?.attributes('aria-current')).toBe('page');

    await wrapper.setProps({ messages: manyMessages(45) });
    await flushPromises();

    expect(pageButtons()[1]?.attributes('aria-current')).toBe('page'); // page "1"
  });
});
