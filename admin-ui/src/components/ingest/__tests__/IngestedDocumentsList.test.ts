import { flushPromises, mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';

import * as adminApi from '../../../services/adminApi';
import IngestedDocumentsList from '../IngestedDocumentsList.vue';

describe('IngestedDocumentsList', () => {
  it('renders a clickable link for a scraped URL and a chunk count', async () => {
    vi.spyOn(adminApi, 'listSectionDocuments').mockResolvedValue([
      {
        source_ref: 'https://example.com/news/1',
        source: 'scrape',
        chunk_count: 3,
      },
    ]);

    const wrapper = mount(IngestedDocumentsList, { props: { sectionId: 1 } });
    await flushPromises();

    const link = wrapper.find('a');
    expect(link.attributes('href')).toBe('https://example.com/news/1');
    expect(link.text()).toBe('https://example.com/news/1');
    expect(wrapper.text()).toContain('Scraping');
    expect(wrapper.text()).toContain('3 blocchi');
  });

  it('renders a non-URL source ref as plain text, not a link', async () => {
    vi.spyOn(adminApi, 'listSectionDocuments').mockResolvedValue([
      { source_ref: 'comunicato.pdf', source: 'manual', chunk_count: 1 },
    ]);

    const wrapper = mount(IngestedDocumentsList, { props: { sectionId: 1 } });
    await flushPromises();

    expect(wrapper.find('a').exists()).toBe(false);
    expect(wrapper.text()).toContain('comunicato.pdf');
    expect(wrapper.text()).toContain('Caricamento manuale');
    expect(wrapper.text()).toContain('1 blocco');
  });

  it('renders an honest empty state when nothing has been ingested', async () => {
    vi.spyOn(adminApi, 'listSectionDocuments').mockResolvedValue([]);

    const wrapper = mount(IngestedDocumentsList, { props: { sectionId: 1 } });
    await flushPromises();

    expect(wrapper.text()).toContain(
      'Nessun contenuto ingerito per questa sezione.',
    );
  });

  it('shows the honest error message from AdminApiError on fetch failure', async () => {
    vi.spyOn(adminApi, 'listSectionDocuments').mockRejectedValue(
      new adminApi.AdminApiError(404, 'section 999 not found'),
    );

    const wrapper = mount(IngestedDocumentsList, { props: { sectionId: 999 } });
    await flushPromises();

    expect(wrapper.text()).toContain('section 999 not found');
  });

  it('refetches when the refresh button is clicked', async () => {
    const listSectionDocumentsSpy = vi
      .spyOn(adminApi, 'listSectionDocuments')
      .mockResolvedValue([]);

    const wrapper = mount(IngestedDocumentsList, { props: { sectionId: 1 } });
    await flushPromises();
    expect(listSectionDocumentsSpy).toHaveBeenCalledTimes(1);

    await wrapper.find('button').trigger('click');
    await flushPromises();
    expect(listSectionDocumentsSpy).toHaveBeenCalledTimes(2);
  });
});
