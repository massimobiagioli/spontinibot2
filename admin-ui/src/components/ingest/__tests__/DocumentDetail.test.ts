import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';

import DocumentDetail from '../DocumentDetail.vue';

describe('DocumentDetail', () => {
  it('renders a clickable link for a scraped URL source', () => {
    const wrapper = mount(DocumentDetail, {
      props: {
        document: {
          source_ref: 'https://example.com/news/1',
          source: 'scrape',
          chunk_count: 3,
          created_at: '2026-07-24 00:00:00',
        },
      },
    });

    const link = wrapper.find('a');
    expect(link.attributes('href')).toBe('https://example.com/news/1');
    expect(link.attributes('target')).toBe('_blank');
    expect(wrapper.text()).toContain('Scraping');
    expect(wrapper.text()).toContain('3');
    expect(wrapper.text()).toContain('2026-07-24 00:00:00');
  });

  it('renders a non-URL source ref as plain text, not a link', () => {
    const wrapper = mount(DocumentDetail, {
      props: {
        document: {
          source_ref: 'comunicato.pdf',
          source: 'manual',
          chunk_count: 1,
          created_at: '2026-07-24 00:00:00',
        },
      },
    });

    expect(wrapper.find('a').exists()).toBe(false);
    expect(wrapper.text()).toContain('comunicato.pdf');
    expect(wrapper.text()).toContain('Caricamento manuale');
  });

  it('emits close when the close button is clicked', async () => {
    const wrapper = mount(DocumentDetail, {
      props: {
        document: {
          source_ref: 'comunicato.pdf',
          source: 'manual',
          chunk_count: 1,
          created_at: '2026-07-24 00:00:00',
        },
      },
    });

    await wrapper.find('button.btn-outline-secondary').trigger('click');

    expect(wrapper.emitted('close')).toBeTruthy();
  });
});
