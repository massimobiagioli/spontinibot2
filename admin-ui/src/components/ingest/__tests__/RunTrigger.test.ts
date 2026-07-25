import { mount } from '@vue/test-utils';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import * as adminApi from '../../../services/adminApi';
import RunTrigger from '../RunTrigger.vue';

describe('RunTrigger', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('triggers a run, polls, and renders pending then done', async () => {
    vi.spyOn(adminApi, 'triggerIngestRun').mockResolvedValue({
      id: 1,
      status: 'pending',
      requested_at: '2026-07-24T00:00:00Z',
    });
    const getRunSpy = vi
      .spyOn(adminApi, 'getIngestRun')
      .mockResolvedValueOnce({
        id: 1,
        status: 'running',
        requested_at: '2026-07-24T00:00:00Z',
      })
      .mockResolvedValueOnce({
        id: 1,
        status: 'done',
        requested_at: '2026-07-24T00:00:00Z',
      });

    const wrapper = mount(RunTrigger);

    await wrapper.find('button').trigger('click');
    await vi.advanceTimersByTimeAsync(0);

    expect(wrapper.text()).toContain('pending');

    await vi.advanceTimersByTimeAsync(2000);
    expect(getRunSpy).toHaveBeenCalledTimes(1);
    expect(wrapper.text()).toContain('running');

    await vi.advanceTimersByTimeAsync(2000);
    expect(getRunSpy).toHaveBeenCalledTimes(2);
    expect(wrapper.text()).toContain('done');

    await vi.advanceTimersByTimeAsync(2000);
    expect(getRunSpy).toHaveBeenCalledTimes(2);
  });

  it('shows an honest error message when the trigger fails', async () => {
    vi.spyOn(adminApi, 'triggerIngestRun').mockRejectedValue(
      new adminApi.AdminApiError(401, 'invalid or missing session cookie'),
    );

    const wrapper = mount(RunTrigger);
    await wrapper.find('button').trigger('click');
    await vi.advanceTimersByTimeAsync(0);

    expect(wrapper.text()).toContain('invalid or missing session cookie');
  });

  it('clears the polling interval on unmount', async () => {
    vi.spyOn(adminApi, 'triggerIngestRun').mockResolvedValue({
      id: 1,
      status: 'pending',
      requested_at: '2026-07-24T00:00:00Z',
    });
    const getRunSpy = vi.spyOn(adminApi, 'getIngestRun').mockResolvedValue({
      id: 1,
      status: 'running',
      requested_at: '2026-07-24T00:00:00Z',
    });

    const wrapper = mount(RunTrigger);
    await wrapper.find('button').trigger('click');
    await vi.advanceTimersByTimeAsync(0);

    wrapper.unmount();

    await vi.advanceTimersByTimeAsync(10000);
    expect(getRunSpy).not.toHaveBeenCalled();
  });
});
