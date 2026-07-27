import { mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';

import * as adminApi from '../../../services/adminApi';
import MessageFeedback from '../MessageFeedback.vue';

describe('MessageFeedback', () => {
  it('submits positive feedback for the full answer with a single click', async () => {
    const createTrainingFeedbackSpy = vi
      .spyOn(adminApi, 'createTrainingFeedback')
      .mockResolvedValue({
        id: 1,
        message_id: 1,
        chunk_id: null,
        answer_span: 'Lo sportello apre alle 9:00.',
        sentiment: 'positive',
        comment: null,
        created_at: '2026-07-24T00:00:00Z',
      });

    const wrapper = mount(MessageFeedback, {
      props: {
        messageId: 1,
        answer: 'Lo sportello apre alle 9:00.',
        initialFeedback: [],
      },
    });

    const positiveButton = wrapper
      .findAll('button')
      .find((b) => b.text() === 'Feedback positivo');
    await positiveButton!.trigger('click');
    await wrapper.vm.$nextTick();
    await Promise.resolve();
    await wrapper.vm.$nextTick();

    expect(createTrainingFeedbackSpy).toHaveBeenCalledWith({
      message_id: 1,
      answer_span: 'Lo sportello apre alle 9:00.',
      sentiment: 'positive',
    });
    expect(wrapper.emitted('changed')).toBeTruthy();
  });

  it('requires a reason before submitting negative feedback', async () => {
    const createTrainingFeedbackSpy = vi.spyOn(
      adminApi,
      'createTrainingFeedback',
    );

    const wrapper = mount(MessageFeedback, {
      props: {
        messageId: 1,
        answer: 'Lo sportello apre alle 9:00.',
        initialFeedback: [],
      },
    });

    const negativeButton = wrapper
      .findAll('button')
      .find((b) => b.text() === 'Feedback negativo');
    await negativeButton!.trigger('click');

    const confirmButton = wrapper
      .findAll('button')
      .find((b) => b.text() === 'Conferma feedback negativo');
    await confirmButton!.trigger('click');

    expect(createTrainingFeedbackSpy).not.toHaveBeenCalled();
    expect(wrapper.text()).toContain('Indica il motivo del feedback negativo.');
  });

  it('submits negative feedback with the given reason as the comment', async () => {
    const createTrainingFeedbackSpy = vi
      .spyOn(adminApi, 'createTrainingFeedback')
      .mockResolvedValue({
        id: 2,
        message_id: 1,
        chunk_id: null,
        answer_span: 'Lo sportello apre alle 9:00.',
        sentiment: 'negative',
        comment: 'orario sbagliato',
        created_at: '2026-07-24T00:00:00Z',
      });

    const wrapper = mount(MessageFeedback, {
      props: {
        messageId: 1,
        answer: 'Lo sportello apre alle 9:00.',
        initialFeedback: [],
      },
    });

    await wrapper
      .findAll('button')
      .find((b) => b.text() === 'Feedback negativo')!
      .trigger('click');
    await wrapper.find('textarea').setValue('orario sbagliato');
    await wrapper
      .findAll('button')
      .find((b) => b.text() === 'Conferma feedback negativo')!
      .trigger('click');
    await wrapper.vm.$nextTick();
    await Promise.resolve();
    await wrapper.vm.$nextTick();

    expect(createTrainingFeedbackSpy).toHaveBeenCalledWith({
      message_id: 1,
      answer_span: 'Lo sportello apre alle 9:00.',
      sentiment: 'negative',
      comment: 'orario sbagliato',
    });
    expect(wrapper.text()).toContain('Negativo');
    expect(wrapper.text()).toContain('orario sbagliato');
  });

  it('shows the honest error message from AdminApiError on submit failure', async () => {
    vi.spyOn(adminApi, 'createTrainingFeedback').mockRejectedValue(
      new adminApi.AdminApiError(500, 'internal error'),
    );

    const wrapper = mount(MessageFeedback, {
      props: { messageId: 1, answer: 'risposta', initialFeedback: [] },
    });

    await wrapper
      .findAll('button')
      .find((b) => b.text() === 'Feedback positivo')!
      .trigger('click');
    await wrapper.vm.$nextTick();
    await Promise.resolve();
    await wrapper.vm.$nextTick();

    expect(wrapper.text()).toContain('internal error');
  });
});
