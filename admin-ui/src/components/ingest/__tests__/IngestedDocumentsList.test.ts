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
        summary: null,
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
        summary: null,
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
        summary: null,
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
        summary: null,
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

  it('shows a summary with the total count and a breakdown by source type', async () => {
    vi.spyOn(adminApi, 'listSectionDocuments').mockResolvedValue([
      {
        source_ref: 'a.pdf',
        source: 'scrape',
        chunk_count: 2,
        created_at: '2026-07-24 00:00:00',
        summary: null,
      },
      {
        source_ref: 'b.pdf',
        source: 'manual',
        chunk_count: 1,
        created_at: '2026-07-25 00:00:00',
        summary: null,
      },
      {
        source_ref: 'c.pdf',
        source: 'manual',
        chunk_count: 1,
        created_at: '2026-07-26 00:00:00',
        summary: null,
      },
    ]);

    const wrapper = mount(IngestedDocumentsList, { props: { sectionId: 1 } });
    await flushPromises();

    expect(wrapper.text()).toContain('3 documenti');
    expect(wrapper.text()).toContain('1 da scraping');
    expect(wrapper.text()).toContain('2 da caricamento manuale');
  });

  it('paginates the list 20 items per page', async () => {
    const docs = Array.from({ length: 45 }, (_, i) => ({
      source_ref: `doc-${i}.pdf`,
      source: 'manual',
      chunk_count: 1,
      created_at: '2026-07-24 00:00:00',
      summary: null,
    }));
    vi.spyOn(adminApi, 'listSectionDocuments').mockResolvedValue(docs);

    const wrapper = mount(IngestedDocumentsList, { props: { sectionId: 1 } });
    await flushPromises();

    expect(wrapper.findAll('.ingested-documents__card')).toHaveLength(20);
    expect(wrapper.text()).toContain('Pagina 1 di 3');
    expect(
      wrapper
        .find('[data-testid="ingested-documents-prev-page"]')
        .attributes('disabled'),
    ).toBeDefined();

    await wrapper
      .find('[data-testid="ingested-documents-next-page"]')
      .trigger('click');
    expect(wrapper.findAll('.ingested-documents__card')).toHaveLength(20);
    expect(wrapper.text()).toContain('Pagina 2 di 3');

    await wrapper
      .find('[data-testid="ingested-documents-next-page"]')
      .trigger('click');
    expect(wrapper.findAll('.ingested-documents__card')).toHaveLength(5);
    expect(wrapper.text()).toContain('Pagina 3 di 3');
    expect(
      wrapper
        .find('[data-testid="ingested-documents-next-page"]')
        .attributes('disabled'),
    ).toBeDefined();

    await wrapper
      .find('[data-testid="ingested-documents-prev-page"]')
      .trigger('click');
    expect(wrapper.text()).toContain('Pagina 2 di 3');
  });

  it('does not show pagination controls when there are 20 or fewer documents', async () => {
    vi.spyOn(adminApi, 'listSectionDocuments').mockResolvedValue([
      {
        source_ref: 'a.pdf',
        source: 'manual',
        chunk_count: 1,
        created_at: '2026-07-24 00:00:00',
        summary: null,
      },
    ]);

    const wrapper = mount(IngestedDocumentsList, { props: { sectionId: 1 } });
    await flushPromises();

    expect(
      wrapper.find('[data-testid="ingested-documents-next-page"]').exists(),
    ).toBe(false);
  });

  it('resets to page 1 when the list is refreshed', async () => {
    const docs = Array.from({ length: 25 }, (_, i) => ({
      source_ref: `doc-${i}.pdf`,
      source: 'manual',
      chunk_count: 1,
      created_at: '2026-07-24 00:00:00',
      summary: null,
    }));
    vi.spyOn(adminApi, 'listSectionDocuments').mockResolvedValue(docs);

    const wrapper = mount(IngestedDocumentsList, { props: { sectionId: 1 } });
    await flushPromises();
    await wrapper
      .find('[data-testid="ingested-documents-next-page"]')
      .trigger('click');
    expect(wrapper.text()).toContain('Pagina 2 di 2');

    const refreshButton = wrapper
      .findAll('button')
      .find((b) => b.text() === 'Aggiorna');
    await refreshButton?.trigger('click');
    await flushPromises();

    expect(wrapper.text()).toContain('Pagina 1 di 2');
  });
});
