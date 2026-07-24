import { mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';

import * as adminApi from '../../../services/adminApi';
import ScheduleEditor from '../ScheduleEditor.vue';

describe('ScheduleEditor', () => {
  it('renders the existing schedule values', () => {
    const wrapper = mount(ScheduleEditor, {
      props: {
        schedule: {
          cron_expr: '0 */4 * * *',
          enabled: true,
          updated_at: '2026-07-24T00:00:00Z',
        },
      },
    });

    const cronInput = wrapper.find('input[type="text"]');
    expect((cronInput.element as HTMLInputElement).value).toBe('0 */4 * * *');
    const enabledCheckbox = wrapper.find('input[type="checkbox"]');
    expect((enabledCheckbox.element as HTMLInputElement).checked).toBe(true);
  });

  it('submits the form and calls upsertSchedule with the edited values', async () => {
    const upsertSpy = vi.spyOn(adminApi, 'upsertSchedule').mockResolvedValue({
      cron_expr: '0 0 * * *',
      enabled: false,
      updated_at: '2026-07-24T01:00:00Z',
    });

    const wrapper = mount(ScheduleEditor, {
      props: { schedule: null },
    });

    await wrapper.find('input[type="text"]').setValue('0 0 * * *');
    await wrapper.find('form').trigger('submit');
    await wrapper.vm.$nextTick();
    await Promise.resolve();

    expect(upsertSpy).toHaveBeenCalledWith('0 0 * * *', false);
    expect(wrapper.emitted('saved')).toHaveLength(1);
    expect(wrapper.emitted('saved')?.[0]?.[0]).toEqual({
      cron_expr: '0 0 * * *',
      enabled: false,
      updated_at: '2026-07-24T01:00:00Z',
    });
  });

  it('shows an honest error message when saving fails', async () => {
    vi.spyOn(adminApi, 'upsertSchedule').mockRejectedValue(
      new adminApi.AdminApiError(500, 'database error: connection refused'),
    );

    const wrapper = mount(ScheduleEditor, { props: { schedule: null } });

    await wrapper.find('form').trigger('submit');
    await wrapper.vm.$nextTick();
    await Promise.resolve();
    await wrapper.vm.$nextTick();

    expect(wrapper.text()).toContain('database error: connection refused');
  });
});
