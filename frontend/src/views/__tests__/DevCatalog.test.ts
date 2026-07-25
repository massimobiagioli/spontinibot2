import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';

import DevCatalog from '../DevCatalog.vue';

describe('DevCatalog', () => {
  it('mounts without error and renders every wrapper section', () => {
    const wrapper = mount(DevCatalog);

    expect(wrapper.findAll('h2').map((h) => h.text())).toEqual([
      'DsButton',
      'DsInput',
      'DsCallout',
      'ChatMessage',
      'ChatInput',
    ]);
  });
});
