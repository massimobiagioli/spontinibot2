import { flushPromises, mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
import { createMemoryHistory, createRouter } from 'vue-router';

import * as adminApi from '../../services/adminApi';
import LoginView from '../LoginView.vue';

function makeRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/login', name: 'login', component: LoginView },
      { path: '/ingest', name: 'ingest', component: { template: '<div />' } },
    ],
  });
}

async function mountAtLogin() {
  const router = makeRouter();
  await router.push('/login');
  await router.isReady();

  return {
    wrapper: mount(LoginView, { global: { plugins: [router] } }),
    router,
  };
}

describe('LoginView', () => {
  it('submits the entered credentials and navigates to /ingest on success', async () => {
    const loginSpy = vi
      .spyOn(adminApi, 'login')
      .mockResolvedValue({ status: 'logged_in' });

    const { wrapper, router } = await mountAtLogin();

    await wrapper.find('input[type="text"]').setValue('operator');
    await wrapper.find('input[type="password"]').setValue('s3cret');
    await wrapper.find('form').trigger('submit.prevent');
    await flushPromises();

    expect(loginSpy).toHaveBeenCalledWith('operator', 's3cret');
    expect(router.currentRoute.value.name).toBe('ingest');
  });

  it('shows an honest error callout when login fails', async () => {
    vi.spyOn(adminApi, 'login').mockRejectedValue(
      new adminApi.AdminApiError(401, 'invalid credentials'),
    );

    const { wrapper, router } = await mountAtLogin();

    await wrapper.find('input[type="text"]').setValue('operator');
    await wrapper.find('input[type="password"]').setValue('wrong');
    await wrapper.find('form').trigger('submit.prevent');
    await flushPromises();

    expect(wrapper.text()).toContain('invalid credentials');
    expect(router.currentRoute.value.name).toBe('login');
  });
});
