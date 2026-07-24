import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import {
  AdminApiError,
  confirmUpload,
  createSection,
  createSource,
  deleteSection,
  deleteSource,
  getIngestConfig,
  getIngestRun,
  getUploadPreview,
  triggerIngestRun,
  uploadDocument,
  upsertSchedule,
} from '../adminApi';

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

function lastCall(fetchMock: ReturnType<typeof vi.fn>): [string, RequestInit] {
  const calls = fetchMock.mock.calls;
  const call = calls[calls.length - 1];
  if (!call) throw new Error('fetch was not called');
  return call as [string, RequestInit];
}

describe('adminApi', () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    fetchMock = vi.fn();
    vi.stubGlobal('fetch', fetchMock);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('getIngestConfig fetches the config and sends the admin key header', async () => {
    const config = { schedule: null, sections: [] };
    fetchMock.mockResolvedValueOnce(jsonResponse(config));

    const result = await getIngestConfig();

    expect(result).toEqual(config);
    const [url, init] = lastCall(fetchMock);
    expect(url).toBe('/admin/api/ingest/config');
    expect((init.headers as Record<string, string>)['X-Admin-Key']).toBe(
      'dev-key',
    );
  });

  it('upsertSchedule PUTs the schedule payload', async () => {
    const schedule = {
      cron_expr: '0 */4 * * *',
      enabled: true,
      updated_at: '2026-07-24T00:00:00Z',
    };
    fetchMock.mockResolvedValueOnce(jsonResponse(schedule));

    const result = await upsertSchedule('0 */4 * * *', true);

    expect(result).toEqual(schedule);
    const [url, init] = lastCall(fetchMock);
    expect(url).toBe('/admin/api/ingest/config/schedule');
    expect(init.method).toBe('PUT');
    expect(JSON.parse(init.body as string)).toEqual({
      cron_expr: '0 */4 * * *',
      enabled: true,
    });
  });

  it('createSection POSTs the section payload', async () => {
    const section = { id: 1, name: 'news', ordering: 10, created_at: 'now' };
    fetchMock.mockResolvedValueOnce(jsonResponse(section, 201));

    const result = await createSection('news', 10);

    expect(result).toEqual(section);
    const [url, init] = lastCall(fetchMock);
    expect(url).toBe('/admin/api/ingest/config/sections');
    expect(init.method).toBe('POST');
  });

  it('deleteSection DELETEs and returns the deleted flag', async () => {
    fetchMock.mockResolvedValueOnce(jsonResponse({ deleted: true }));

    const result = await deleteSection(1);

    expect(result).toBe(true);
    const [url, init] = lastCall(fetchMock);
    expect(url).toBe('/admin/api/ingest/config/sections/1');
    expect(init.method).toBe('DELETE');
  });

  it('createSource POSTs to the section-scoped sources endpoint', async () => {
    const source = {
      id: 1,
      section_id: 10,
      source_type: 'scrape',
      url: 'https://example.com',
      enabled: true,
      created_at: 'now',
      coming_soon: false,
    };
    fetchMock.mockResolvedValueOnce(jsonResponse(source, 201));

    const result = await createSource(
      10,
      'scrape',
      'https://example.com',
      true,
    );

    expect(result).toEqual(source);
    const [url, init] = lastCall(fetchMock);
    expect(url).toBe('/admin/api/ingest/config/sources?section_id=10');
    expect(JSON.parse(init.body as string)).toEqual({
      source_type: 'scrape',
      url: 'https://example.com',
      enabled: true,
    });
  });

  it('deleteSource DELETEs and returns the deleted flag', async () => {
    fetchMock.mockResolvedValueOnce(jsonResponse({ deleted: true }));

    const result = await deleteSource(2);

    expect(result).toBe(true);
    const [url, init] = lastCall(fetchMock);
    expect(url).toBe('/admin/api/ingest/config/sources/2');
    expect(init.method).toBe('DELETE');
  });

  it('triggerIngestRun POSTs to the run endpoint', async () => {
    const run = { id: 1, status: 'pending', requested_at: 'now' };
    fetchMock.mockResolvedValueOnce(jsonResponse(run, 202));

    const result = await triggerIngestRun();

    expect(result).toEqual(run);
    const [url, init] = lastCall(fetchMock);
    expect(url).toBe('/admin/api/ingest/run');
    expect(init.method).toBe('POST');
  });

  it('getIngestRun fetches the run status by id', async () => {
    const run = { id: 1, status: 'done', requested_at: 'now' };
    fetchMock.mockResolvedValueOnce(jsonResponse(run));

    const result = await getIngestRun(1);

    expect(result).toEqual(run);
    expect(lastCall(fetchMock)[0]).toBe('/admin/api/ingest/run/1');
  });

  it('uploadDocument POSTs a multipart form with the file and metadata', async () => {
    const uploadResponse = {
      token: 'abc',
      preview_url: '/admin/api/upload/preview/abc',
    };
    fetchMock.mockResolvedValueOnce(jsonResponse(uploadResponse, 201));
    const file = new File(['content'], 'doc.txt', { type: 'text/plain' });

    const result = await uploadDocument(file, 'news', {
      category: 'general',
      tags: ['a', 'b'],
      trustScore: 0.8,
    });

    expect(result).toEqual(uploadResponse);
    const [url, init] = lastCall(fetchMock);
    expect(url).toBe('/admin/api/upload');
    expect(init.method).toBe('POST');
    const form = init.body as FormData;
    expect(form.get('section')).toBe('news');
    expect(form.get('category')).toBe('general');
    expect(form.get('tags')).toBe('a,b');
    expect(form.get('trust_score')).toBe('0.8');
  });

  it('getUploadPreview fetches the preview by token', async () => {
    const preview = {
      extracted_text: 'hello',
      format: 'txt',
      byte_size: 5,
      section: 'news',
      filename: 'doc.txt',
      metadata: { category: null, tags: null, trust_score: null },
      chunk_count_estimate: 1,
    };
    fetchMock.mockResolvedValueOnce(jsonResponse(preview));

    const result = await getUploadPreview('abc');

    expect(result).toEqual(preview);
    expect(lastCall(fetchMock)[0]).toBe('/admin/api/upload/preview/abc');
  });

  it('confirmUpload POSTs to the confirm endpoint by token', async () => {
    const confirmed = { document_ids: [1, 2], chunk_count: 2 };
    fetchMock.mockResolvedValueOnce(jsonResponse(confirmed));

    const result = await confirmUpload('abc');

    expect(result).toEqual(confirmed);
    const [url, init] = lastCall(fetchMock);
    expect(url).toBe('/admin/api/upload/confirm/abc');
    expect(init.method).toBe('POST');
  });

  it('throws AdminApiError with the parsed message on a non-2xx response', async () => {
    fetchMock.mockImplementation(() =>
      Promise.resolve(
        jsonResponse({ error: 'invalid or missing X-Admin-Key header' }, 401),
      ),
    );

    const error = await getIngestConfig().catch((e: unknown) => e);

    expect(error).toBeInstanceOf(AdminApiError);
    expect((error as AdminApiError).status).toBe(401);
    expect((error as AdminApiError).message).toBe(
      'invalid or missing X-Admin-Key header',
    );
  });

  it('falls back to a generic message when the error body is not JSON', async () => {
    fetchMock.mockResolvedValueOnce(new Response('not json', { status: 500 }));

    await expect(getIngestConfig()).rejects.toMatchObject({
      status: 500,
    });
  });
});
