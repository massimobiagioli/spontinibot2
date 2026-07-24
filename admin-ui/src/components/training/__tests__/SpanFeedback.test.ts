import { flushPromises, mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';

import * as adminApi from '../../../services/adminApi';
import SpanFeedback from '../SpanFeedback.vue';

const answer = 'Lo sportello apre alle 9:00. Il sabato è chiuso.';

describe('SpanFeedback', () => {
  it('splits the answer into sentence segments and reveals the form on click', async () => {
    vi.spyOn(adminApi, 'listTrainingFeedback').mockResolvedValue([]);

    const wrapper = mount(SpanFeedback, {
      props: { messageId: 1, answer },
    });
    await flushPromises();

    const segments = wrapper.findAll('.span-feedback__segment');
    expect(segments.length).toBe(2);
    expect(segments[0]?.text()).toBe('Lo sportello apre alle 9:00.');
    expect(segments[1]?.text()).toBe('Il sabato è chiuso.');

    expect(wrapper.find('textarea').exists()).toBe(false);

    await segments[0]?.trigger('click');

    expect(wrapper.find('textarea').exists()).toBe(true);
    expect(segments[0]?.attributes('aria-pressed')).toBe('true');
  });

  it('renders normally with an empty feedback list when the initial fetch fails', async () => {
    vi.spyOn(adminApi, 'listTrainingFeedback').mockRejectedValue(
      new adminApi.AdminApiError(500, 'internal error'),
    );

    const wrapper = mount(SpanFeedback, {
      props: { messageId: 1, answer },
    });
    await flushPromises();

    expect(wrapper.find('.span-feedback__segment').exists()).toBe(true);
    expect(wrapper.find('ul').exists()).toBe(false);
    expect(wrapper.text()).not.toContain('internal error');
  });

  it('toggles a segment off when clicked again, hiding the form', async () => {
    vi.spyOn(adminApi, 'listTrainingFeedback').mockResolvedValue([]);

    const wrapper = mount(SpanFeedback, {
      props: { messageId: 1, answer },
    });
    await flushPromises();

    const segment = wrapper.findAll('.span-feedback__segment')[0];
    if (!segment) throw new Error('segment not found');
    await segment.trigger('click');
    expect(wrapper.find('textarea').exists()).toBe(true);

    await segment.trigger('click');
    expect(wrapper.find('textarea').exists()).toBe(false);
  });

  it('submits negative feedback with the exact segment text and comment', async () => {
    vi.spyOn(adminApi, 'listTrainingFeedback').mockResolvedValue([]);
    const createTrainingFeedbackSpy = vi
      .spyOn(adminApi, 'createTrainingFeedback')
      .mockResolvedValue({
        id: 1,
        message_id: 1,
        chunk_id: null,
        answer_span: 'Lo sportello apre alle 9:00.',
        sentiment: 'negative',
        comment: 'orario sbagliato',
        created_at: '2026-07-24T00:00:00Z',
      });

    const wrapper = mount(SpanFeedback, {
      props: { messageId: 1, answer },
    });
    await flushPromises();

    const segment = wrapper.findAll('.span-feedback__segment')[0];
    if (!segment) throw new Error('segment not found');
    await segment.trigger('click');
    await wrapper.find('textarea').setValue('orario sbagliato');

    const negativeButton = wrapper
      .findAll('button')
      .find((b) => b.text() === 'Feedback negativo');
    if (!negativeButton) throw new Error('negative button not found');
    await negativeButton.trigger('click');
    await flushPromises();

    expect(createTrainingFeedbackSpy).toHaveBeenCalledWith({
      message_id: 1,
      answer_span: 'Lo sportello apre alle 9:00.',
      sentiment: 'negative',
      comment: 'orario sbagliato',
    });
    expect(wrapper.text()).toContain('Negativo');
    expect(wrapper.text()).toContain('orario sbagliato');
    expect(wrapper.find('textarea').exists()).toBe(false);
  });

  it('submits positive feedback without a comment field in the payload', async () => {
    vi.spyOn(adminApi, 'listTrainingFeedback').mockResolvedValue([]);
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

    const wrapper = mount(SpanFeedback, {
      props: { messageId: 1, answer },
    });
    await flushPromises();

    const segment = wrapper.findAll('.span-feedback__segment')[0];
    if (!segment) throw new Error('segment not found');
    await segment.trigger('click');

    const positiveButton = wrapper
      .findAll('button')
      .find((b) => b.text() === 'Feedback positivo');
    if (!positiveButton) throw new Error('positive button not found');
    await positiveButton.trigger('click');
    await flushPromises();

    expect(createTrainingFeedbackSpy).toHaveBeenCalledWith({
      message_id: 1,
      answer_span: 'Lo sportello apre alle 9:00.',
      sentiment: 'positive',
    });
  });

  it('renders previously persisted feedback fetched on mount', async () => {
    vi.spyOn(adminApi, 'listTrainingFeedback').mockResolvedValue([
      {
        id: 5,
        message_id: 1,
        chunk_id: null,
        answer_span: 'Il sabato è chiuso.',
        sentiment: 'positive',
        comment: null,
        created_at: '2026-07-24T00:00:00Z',
      },
    ]);

    const wrapper = mount(SpanFeedback, {
      props: { messageId: 1, answer },
    });
    await flushPromises();

    expect(wrapper.text()).toContain('Il sabato è chiuso.');
    expect(wrapper.text()).toContain('Positivo');
  });

  it('shows the honest error message from AdminApiError on submit failure', async () => {
    vi.spyOn(adminApi, 'listTrainingFeedback').mockResolvedValue([]);
    vi.spyOn(adminApi, 'createTrainingFeedback').mockRejectedValue(
      new adminApi.AdminApiError(400, 'invalid sentiment: neutral'),
    );

    const wrapper = mount(SpanFeedback, {
      props: { messageId: 1, answer },
    });
    await flushPromises();

    const segment = wrapper.findAll('.span-feedback__segment')[0];
    if (!segment) throw new Error('segment not found');
    await segment.trigger('click');

    const negativeButton = wrapper
      .findAll('button')
      .find((b) => b.text() === 'Feedback negativo');
    if (!negativeButton) throw new Error('negative button not found');
    await negativeButton.trigger('click');
    await flushPromises();

    expect(wrapper.text()).toContain('invalid sentiment: neutral');
  });
});
