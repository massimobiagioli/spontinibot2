import { mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';

import * as adminApi from '../../../services/adminApi';
import QuestionDetail from '../QuestionDetail.vue';

const chatMessage: adminApi.TrainingMessageResponse = {
  id: 1,
  session_id: 1,
  question: "A che ora apre l'anagrafe?",
  answer: 'Lo sportello apre alle 9:00.',
  sources: [{ document_id: 7, source_ref: 'orari.md' }],
  fell_back: false,
  created_at: '2026-07-24T00:00:00Z',
  expected_answer: 'Dalle 9:00',
  execution_time_ms: 84,
  source: 'chat',
};

describe('QuestionDetail', () => {
  it('renders the question, expected answer, bot answer, execution time, and sources', () => {
    vi.spyOn(adminApi, 'listTrainingFeedback').mockResolvedValue([]);

    const wrapper = mount(QuestionDetail, {
      props: { message: chatMessage, feedback: [] },
    });

    expect(wrapper.text()).toContain("A che ora apre l'anagrafe?");
    expect(wrapper.text()).toContain('Dalle 9:00');
    expect(wrapper.text()).toContain('Lo sportello apre alle 9:00.');
    expect(wrapper.text()).toContain('84 ms');
    expect(wrapper.find('details summary').text()).toContain('Fonti (1)');
    expect(wrapper.text()).toContain('orari.md');
  });

  it('shows "inserita manualmente" instead of a duration for manual entries', () => {
    vi.spyOn(adminApi, 'listTrainingFeedback').mockResolvedValue([]);

    const wrapper = mount(QuestionDetail, {
      props: {
        message: { ...chatMessage, execution_time_ms: null, source: 'manual' },
        feedback: [],
      },
    });

    expect(wrapper.text()).toContain('n/d (inserita manualmente)');
    expect(wrapper.text()).toContain('Manuale');
  });

  it('shows the fallback callout instead of sources when the bot found nothing', () => {
    vi.spyOn(adminApi, 'listTrainingFeedback').mockResolvedValue([]);

    const wrapper = mount(QuestionDetail, {
      props: {
        message: { ...chatMessage, fell_back: true, sources: [] },
        feedback: [],
      },
    });

    expect(wrapper.text()).toContain('Nessuna informazione trovata');
    expect(wrapper.find('details').exists()).toBe(false);
  });

  it('renders a URL source as a clickable link and a non-URL one as plain text', () => {
    vi.spyOn(adminApi, 'listTrainingFeedback').mockResolvedValue([]);

    const wrapper = mount(QuestionDetail, {
      props: {
        message: {
          ...chatMessage,
          sources: [
            { document_id: 7, source_ref: 'https://example.com/orari' },
            { document_id: 8, source_ref: 'Regolamento comunale' },
          ],
        },
        feedback: [],
      },
    });

    const link = wrapper.find('a[href="https://example.com/orari"]');
    expect(link.exists()).toBe(true);
    expect(link.attributes('target')).toBe('_blank');
    expect(wrapper.findAll('a').length).toBe(1);
    expect(wrapper.text()).toContain('Regolamento comunale');
  });

  it('emits close when the close button is clicked', async () => {
    vi.spyOn(adminApi, 'listTrainingFeedback').mockResolvedValue([]);

    const wrapper = mount(QuestionDetail, {
      props: { message: chatMessage, feedback: [] },
    });

    await wrapper.find('button.btn-outline-secondary').trigger('click');

    expect(wrapper.emitted('close')).toBeTruthy();
  });

  it('re-emits feedback-changed from the nested MessageFeedback with the message id', async () => {
    vi.spyOn(adminApi, 'createTrainingFeedback').mockResolvedValue({
      id: 9,
      message_id: 1,
      chunk_id: null,
      answer_span: chatMessage.answer,
      sentiment: 'positive',
      comment: null,
      created_at: '2026-07-24T00:00:00Z',
    });

    const wrapper = mount(QuestionDetail, {
      props: { message: chatMessage, feedback: [] },
    });

    await wrapper
      .findAll('button')
      .find((b) => b.text() === 'Feedback positivo')!
      .trigger('click');
    await wrapper.vm.$nextTick();
    await Promise.resolve();
    await wrapper.vm.$nextTick();

    const emitted = wrapper.emitted('feedback-changed');
    expect(emitted).toBeTruthy();
    expect(emitted![0]![0]).toBe(1);
  });
});
