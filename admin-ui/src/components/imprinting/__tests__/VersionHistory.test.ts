import { flushPromises, mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';

import * as adminApi from '../../../services/adminApi';
import VersionHistory from '../VersionHistory.vue';

const versions: adminApi.PersonaResponse[] = [
  {
    id: 2,
    version: 2,
    name: 'gaspare',
    system_prompt: 'Sei Gaspare, versione due.',
    tone: null,
    fallback_message: null,
    is_active: true,
    created_at: '2026-07-24T00:00:00Z',
    created_by: 'admin',
  },
  {
    id: 1,
    version: 1,
    name: 'gaspare',
    system_prompt: 'Sei Gaspare, versione uno.',
    tone: null,
    fallback_message: null,
    is_active: false,
    created_at: '2026-07-20T00:00:00Z',
    created_by: 'admin',
  },
];

describe('VersionHistory', () => {
  it('renders every version with the active one badged', () => {
    const wrapper = mount(VersionHistory, { props: { versions } });

    expect(wrapper.text()).toContain('v2');
    expect(wrapper.text()).toContain('v1');
    expect(wrapper.text()).toContain('Attiva');
    // Only the non-active version offers activate/delete buttons.
    expect(wrapper.findAll('li button').length).toBe(2);
  });

  it('opens the confirm dialog before calling activatePersona, and refreshes on confirm', async () => {
    const activatePersonaSpy = vi
      .spyOn(adminApi, 'activatePersona')
      .mockResolvedValue({ status: 'activated' });

    const wrapper = mount(VersionHistory, { props: { versions } });

    await wrapper.find('button').trigger('click');
    expect(activatePersonaSpy).not.toHaveBeenCalled();

    const dialog = wrapper.find('[data-testid="activate-dialog"]');
    expect(dialog.attributes('open')).toBeDefined();

    await dialog.find('button.btn-danger').trigger('click');
    await flushPromises();

    expect(activatePersonaSpy).toHaveBeenCalledWith(1);
    expect(wrapper.emitted('changed')).toBeTruthy();
  });

  it('shows the honest error message from AdminApiError on activation failure', async () => {
    vi.spyOn(adminApi, 'activatePersona').mockRejectedValue(
      new adminApi.AdminApiError(500, 'internal error'),
    );

    const wrapper = mount(VersionHistory, { props: { versions } });

    await wrapper.find('button').trigger('click');
    await wrapper
      .find('[data-testid="activate-dialog"]')
      .find('button.btn-danger')
      .trigger('click');
    await flushPromises();

    expect(wrapper.text()).toContain('internal error');
  });

  it('opens the delete confirm dialog before calling deletePersonaVersion, and refreshes on confirm', async () => {
    const deletePersonaVersionSpy = vi
      .spyOn(adminApi, 'deletePersonaVersion')
      .mockResolvedValue({ status: 'deleted' });

    const wrapper = mount(VersionHistory, { props: { versions } });

    await wrapper.findAll('li button')[1]!.trigger('click');
    expect(deletePersonaVersionSpy).not.toHaveBeenCalled();

    const dialog = wrapper.find('[data-testid="delete-dialog"]');
    expect(dialog.attributes('open')).toBeDefined();

    await dialog.find('button.btn-danger').trigger('click');
    await flushPromises();

    expect(deletePersonaVersionSpy).toHaveBeenCalledWith(1);
    expect(wrapper.emitted('changed')).toBeTruthy();
  });

  it('shows the honest error message from AdminApiError on delete failure', async () => {
    vi.spyOn(adminApi, 'deletePersonaVersion').mockRejectedValue(
      new adminApi.AdminApiError(409, 'cannot delete the active persona version'),
    );

    const wrapper = mount(VersionHistory, { props: { versions } });

    await wrapper.findAll('li button')[1]!.trigger('click');
    await wrapper
      .find('[data-testid="delete-dialog"]')
      .find('button.btn-danger')
      .trigger('click');
    await flushPromises();

    expect(wrapper.text()).toContain('cannot delete the active persona version');
  });
});
