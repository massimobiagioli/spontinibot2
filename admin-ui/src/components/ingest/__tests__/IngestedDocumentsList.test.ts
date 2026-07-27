import { flushPromises, mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';

import * as adminApi from '../../../services/adminApi';
import IngestedDocumentsList from '../IngestedDocumentsList.vue';

describe('IngestedDocumentsList', () => {
  it('renders a card with source label, chunk count, and ingestion date', async () => {
    vi.spyOn(adminApi, 'listSectionDocuments').mockResolvedValue([
      {
        source_ref: 'https://example.com/news/1',
        source: 'scrape',
        chunk_count: 3,
        created_at: '2026-07-24 00:00:00',
      },
    ]);

    const wrapper = mount(IngestedDocumentsList, { props: { sectionId: 1 } });
    await flushPromises();

    expect(wrapper.text()).toContain('https://example.com/news/1');
    expect(wrapper.text()).toContain('Scraping');
    expect(wrapper.text()).toContain('3 blocchi');
    expect(wrapper.text()).toContain('2026-07-24 00:00:00');
  });

  it('renders a manual upload label and singular chunk count', async () => {
    vi.spyOn(adminApi, 'listSectionDocuments').mockResolvedValue([
      {
        source_ref: 'comunicato.pdf',
        source: 'manual',
        chunk_count: 1,
        created_at: '2026-07-24 00:00:00',
      },
    ]);

    const wrapper = mount(IngestedDocumentsList, { props: { sectionId: 1 } });
    await flushPromises();

    expect(wrapper.text()).toContain('comunicato.pdf');
    expect(wrapper.text()).toContain('Caricamento manuale');
    expect(wrapper.text()).toContain('1 blocco');
  });

  it('opens a detail dialog with a clickable link when a card is clicked', async () => {
    vi.spyOn(adminApi, 'listSectionDocuments').mockResolvedValue([
      {
        source_ref: 'https://example.com/news/1',
        source: 'scrape',
        chunk_count: 3,
        created_at: '2026-07-24 00:00:00',
      },
    ]);

    const wrapper = mount(IngestedDocumentsList, { props: { sectionId: 1 } });
    await flushPromises();

    expect(wrapper.find('dialog').exists()).toBe(false);

    await wrapper.find('.ingested-documents__card').trigger('click');

    const dialog = wrapper.find('dialog');
    expect(dialog.exists()).toBe(true);
    const link = dialog.find('a');
    expect(link.attributes('href')).toBe('https://example.com/news/1');
  });

  it('opens a detail dialog rendering a non-URL source ref as plain text', async () => {
    vi.spyOn(adminApi, 'listSectionDocuments').mockResolvedValue([
      {
        source_ref: 'comunicato.pdf',
        source: 'manual',
        chunk_count: 1,
        created_at: '2026-07-24 00:00:00',
      },
    ]);

    const wrapper = mount(IngestedDocumentsList, { props: { sectionId: 1 } });
    await flushPromises();

    await wrapper.find('.ingested-documents__card').trigger('click');

    const dialog = wrapper.find('dialog');
    expect(dialog.find('a').exists()).toBe(false);
    expect(dialog.text()).toContain('comunicato.pdf');
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
