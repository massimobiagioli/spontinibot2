import { flushPromises, mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';

import * as adminApi from '../../../services/adminApi';
import CloseSessionForm from '../CloseSessionForm.vue';

describe('CloseSessionForm', () => {
  it('opens the confirm dialog before calling closeSession, and passes trimmed notes', async () => {
    const closeSessionSpy = vi
      .spyOn(adminApi, 'closeSession')
      .mockResolvedValue({ closed: true });

    const wrapper = mount(CloseSessionForm, { props: { sessionId: 1 } });

    await wrapper.find('textarea').setValue('  tutto ok  ');
    await wrapper.find('button.btn-danger').trigger('click');
    expect(closeSessionSpy).not.toHaveBeenCalled();

    const dialog = wrapper.find('[data-testid="close-session-dialog"]');
    expect(dialog.attributes('open')).toBeDefined();

    await dialog.find('button.btn-danger').trigger('click');
    await flushPromises();

    expect(closeSessionSpy).toHaveBeenCalledWith(1, 'tutto ok');
    expect(wrapper.emitted('closed')).toBeTruthy();
  });

  it('passes undefined notes when the field is left empty', async () => {
    const closeSessionSpy = vi
      .spyOn(adminApi, 'closeSession')
      .mockResolvedValue({ closed: true });

    const wrapper = mount(CloseSessionForm, { props: { sessionId: 1 } });

    await wrapper.find('button.btn-danger').trigger('click');
    await wrapper
      .find('[data-testid="close-session-dialog"]')
      .find('button.btn-danger')
      .trigger('click');
    await flushPromises();

    expect(closeSessionSpy).toHaveBeenCalledWith(1, undefined);
  });

  it('does not call closeSession when the dialog is cancelled', async () => {
    const closeSessionSpy = vi.spyOn(adminApi, 'closeSession');

    const wrapper = mount(CloseSessionForm, { props: { sessionId: 1 } });

    await wrapper.find('button.btn-danger').trigger('click');
    await wrapper
      .find('[data-testid="close-session-dialog"]')
      .find('button.btn-outline-secondary')
      .trigger('click');

    expect(closeSessionSpy).not.toHaveBeenCalled();
  });

  it('shows the honest error message from AdminApiError on failure', async () => {
    vi.spyOn(adminApi, 'closeSession').mockRejectedValue(
      new adminApi.AdminApiError(500, 'internal error'),
    );

    const wrapper = mount(CloseSessionForm, { props: { sessionId: 1 } });

    await wrapper.find('button.btn-danger').trigger('click');
    await wrapper
      .find('[data-testid="close-session-dialog"]')
      .find('button.btn-danger')
      .trigger('click');
    await flushPromises();

    expect(wrapper.text()).toContain('internal error');
  });
});
