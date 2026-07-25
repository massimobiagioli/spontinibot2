import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import {
  AdminApiError,
  activatePersona,
  askTrainingMessage,
  closeSession,
  confirmUpload,
  createPersona,
  createSection,
  createSession,
  createSource,
  createTrainingFeedback,
  deleteSection,
  deleteSource,
  getIngestConfig,
  getIngestRun,
  getPersonaVersions,
  getSession,
  getUploadPreview,
  listSessions,
  listTrainingFeedback,
  listTrainingMessages,
  login,
  logout,
  reloadPersona,
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

  it('getIngestConfig fetches the config and sends the session cookie', async () => {
    const config = { schedule: null, sections: [] };
    fetchMock.mockResolvedValueOnce(jsonResponse(config));

    const result = await getIngestConfig();

    expect(result).toEqual(config);
    const [url, init] = lastCall(fetchMock);
    expect(url).toBe('/admin/api/ingest/config');
    expect(init.credentials).toBe('include');
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

  it('getPersonaVersions fetches versions scoped by name', async () => {
    const versions = [
      {
        id: 1,
        version: 1,
        name: 'gaspare',
        system_prompt: 'Sei Gaspare.',
        tone: null,
        fallback_message: null,
        is_active: true,
        created_at: 'now',
        created_by: 'admin',
      },
    ];
    fetchMock.mockResolvedValueOnce(jsonResponse(versions));

    const result = await getPersonaVersions('gaspare');

    expect(result).toEqual(versions);
    expect(lastCall(fetchMock)[0]).toBe('/admin/api/persona?name=gaspare');
  });

  it('createPersona POSTs the persona payload', async () => {
    const persona = {
      id: 2,
      version: 2,
      name: 'gaspare',
      system_prompt: 'Sei Gaspare, versione due.',
      tone: 'cordiale',
      fallback_message: 'Non lo so.',
      is_active: true,
      created_at: 'now',
      created_by: 'admin',
    };
    fetchMock.mockResolvedValueOnce(jsonResponse(persona, 201));

    const result = await createPersona({
      name: 'gaspare',
      system_prompt: 'Sei Gaspare, versione due.',
      tone: 'cordiale',
      fallback_message: 'Non lo so.',
      activate: true,
    });

    expect(result).toEqual(persona);
    const [url, init] = lastCall(fetchMock);
    expect(url).toBe('/admin/api/persona');
    expect(init.method).toBe('POST');
    expect(JSON.parse(init.body as string)).toEqual({
      name: 'gaspare',
      system_prompt: 'Sei Gaspare, versione due.',
      tone: 'cordiale',
      fallback_message: 'Non lo so.',
      activate: true,
    });
  });

  it('activatePersona POSTs to the activate endpoint by id', async () => {
    fetchMock.mockResolvedValueOnce(jsonResponse({ status: 'activated' }));

    const result = await activatePersona(2);

    expect(result).toEqual({ status: 'activated' });
    const [url, init] = lastCall(fetchMock);
    expect(url).toBe('/admin/api/persona/2/activate');
    expect(init.method).toBe('POST');
  });

  it('reloadPersona POSTs to the reload endpoint', async () => {
    fetchMock.mockResolvedValueOnce(jsonResponse({ status: 'reloaded' }));

    const result = await reloadPersona();

    expect(result).toEqual({ status: 'reloaded' });
    const [url, init] = lastCall(fetchMock);
    expect(url).toBe('/admin/api/persona/reload');
    expect(init.method).toBe('POST');
  });

  it('createSession POSTs the session payload with an optional created_by', async () => {
    const session = {
      id: 1,
      title: 'Sessione di prova',
      created_at: 'now',
      created_by: 'operator1',
      closed_at: null,
    };
    fetchMock.mockResolvedValueOnce(jsonResponse(session, 201));

    const result = await createSession('Sessione di prova', 'operator1');

    expect(result).toEqual(session);
    const [url, init] = lastCall(fetchMock);
    expect(url).toBe('/admin/api/training/sessions');
    expect(init.method).toBe('POST');
    expect(JSON.parse(init.body as string)).toEqual({
      title: 'Sessione di prova',
      created_by: 'operator1',
    });
  });

  it('createSession omits created_by when not provided', async () => {
    fetchMock.mockResolvedValueOnce(
      jsonResponse(
        {
          id: 1,
          title: 'Sessione',
          created_at: 'now',
          created_by: null,
          closed_at: null,
        },
        201,
      ),
    );

    await createSession('Sessione');

    const [, init] = lastCall(fetchMock);
    expect(JSON.parse(init.body as string)).toEqual({ title: 'Sessione' });
  });

  it('createSession throws AdminApiError on a non-2xx response', async () => {
    fetchMock.mockResolvedValueOnce(
      jsonResponse({ error: 'invalid title' }, 400),
    );

    const error = await createSession('').catch((e: unknown) => e);

    expect(error).toBeInstanceOf(AdminApiError);
    expect((error as AdminApiError).status).toBe(400);
  });

  it('listSessions fetches all sessions', async () => {
    const sessions = [
      {
        id: 1,
        title: 'Sessione',
        created_at: 'now',
        created_by: null,
        closed_at: null,
      },
    ];
    fetchMock.mockResolvedValueOnce(jsonResponse(sessions));

    const result = await listSessions();

    expect(result).toEqual(sessions);
    expect(lastCall(fetchMock)[0]).toBe('/admin/api/training/sessions');
  });

  it('listSessions throws AdminApiError on a non-2xx response', async () => {
    fetchMock.mockResolvedValueOnce(
      jsonResponse({ error: 'server error' }, 500),
    );

    const error = await listSessions().catch((e: unknown) => e);

    expect(error).toBeInstanceOf(AdminApiError);
    expect((error as AdminApiError).status).toBe(500);
  });

  it('getSession fetches a session by id', async () => {
    const session = {
      id: 1,
      title: 'Sessione',
      created_at: 'now',
      created_by: null,
      closed_at: null,
    };
    fetchMock.mockResolvedValueOnce(jsonResponse(session));

    const result = await getSession(1);

    expect(result).toEqual(session);
    expect(lastCall(fetchMock)[0]).toBe('/admin/api/training/sessions/1');
  });

  it('getSession throws AdminApiError on a non-2xx response', async () => {
    fetchMock.mockResolvedValueOnce(
      jsonResponse({ error: 'training session 999 not found' }, 404),
    );

    const error = await getSession(999).catch((e: unknown) => e);

    expect(error).toBeInstanceOf(AdminApiError);
    expect((error as AdminApiError).status).toBe(404);
  });

  it('closeSession POSTs to the close endpoint by id', async () => {
    fetchMock.mockResolvedValueOnce(jsonResponse({ closed: true }));

    const result = await closeSession(1);

    expect(result).toEqual({ closed: true });
    const [url, init] = lastCall(fetchMock);
    expect(url).toBe('/admin/api/training/sessions/1/close');
    expect(init.method).toBe('POST');
  });

  it('closeSession throws AdminApiError on a non-2xx response', async () => {
    fetchMock.mockResolvedValueOnce(
      jsonResponse({ error: 'server error' }, 500),
    );

    const error = await closeSession(1).catch((e: unknown) => e);

    expect(error).toBeInstanceOf(AdminApiError);
    expect((error as AdminApiError).status).toBe(500);
  });

  it('askTrainingMessage POSTs the question to the session messages endpoint', async () => {
    const message = {
      id: 1,
      session_id: 1,
      question: "A che ora apre l'anagrafe?",
      answer: 'Lo sportello apre alle 9:00.',
      sources: [{ document_id: 7, source_ref: 'orari.md' }],
      fell_back: false,
      created_at: 'now',
    };
    fetchMock.mockResolvedValueOnce(jsonResponse(message, 201));

    const result = await askTrainingMessage(1, "A che ora apre l'anagrafe?");

    expect(result).toEqual(message);
    const [url, init] = lastCall(fetchMock);
    expect(url).toBe('/admin/api/training/sessions/1/messages');
    expect(init.method).toBe('POST');
    expect(JSON.parse(init.body as string)).toEqual({
      question: "A che ora apre l'anagrafe?",
    });
  });

  it('askTrainingMessage throws AdminApiError on a non-2xx response', async () => {
    fetchMock.mockResolvedValueOnce(
      jsonResponse({ error: 'generation service error: timeout' }, 502),
    );

    const error = await askTrainingMessage(1, 'domanda').catch(
      (e: unknown) => e,
    );

    expect(error).toBeInstanceOf(AdminApiError);
    expect((error as AdminApiError).status).toBe(502);
  });

  it('listTrainingMessages fetches messages for a session', async () => {
    const messages = [
      {
        id: 1,
        session_id: 1,
        question: "A che ora apre l'anagrafe?",
        answer: 'Lo sportello apre alle 9:00.',
        sources: [],
        fell_back: false,
        created_at: 'now',
      },
    ];
    fetchMock.mockResolvedValueOnce(jsonResponse(messages));

    const result = await listTrainingMessages(1);

    expect(result).toEqual(messages);
    expect(lastCall(fetchMock)[0]).toBe(
      '/admin/api/training/sessions/1/messages',
    );
  });

  it('listTrainingMessages throws AdminApiError on a non-2xx response', async () => {
    fetchMock.mockResolvedValueOnce(
      jsonResponse({ error: 'server error' }, 500),
    );

    const error = await listTrainingMessages(1).catch((e: unknown) => e);

    expect(error).toBeInstanceOf(AdminApiError);
    expect((error as AdminApiError).status).toBe(500);
  });

  it('createTrainingFeedback POSTs the feedback payload', async () => {
    const feedback = {
      id: 1,
      message_id: 1,
      chunk_id: null,
      answer_span: 'alle 9:00',
      sentiment: 'negative',
      comment: 'orario sbagliato',
      created_at: 'now',
    };
    fetchMock.mockResolvedValueOnce(jsonResponse(feedback, 201));

    const result = await createTrainingFeedback({
      message_id: 1,
      answer_span: 'alle 9:00',
      sentiment: 'negative',
      comment: 'orario sbagliato',
    });

    expect(result).toEqual(feedback);
    const [url, init] = lastCall(fetchMock);
    expect(url).toBe('/admin/api/training/feedback');
    expect(init.method).toBe('POST');
    expect(JSON.parse(init.body as string)).toEqual({
      message_id: 1,
      answer_span: 'alle 9:00',
      sentiment: 'negative',
      comment: 'orario sbagliato',
    });
  });

  it('createTrainingFeedback throws AdminApiError on a non-2xx response', async () => {
    fetchMock.mockResolvedValueOnce(
      jsonResponse({ error: 'invalid sentiment: neutral' }, 400),
    );

    const error = await createTrainingFeedback({
      message_id: 1,
      answer_span: 'alle 9:00',
      sentiment: 'negative',
    }).catch((e: unknown) => e);

    expect(error).toBeInstanceOf(AdminApiError);
    expect((error as AdminApiError).status).toBe(400);
  });

  it('listTrainingFeedback fetches feedback for a message', async () => {
    const feedback = [
      {
        id: 1,
        message_id: 1,
        chunk_id: null,
        answer_span: 'alle 9:00',
        sentiment: 'positive',
        comment: null,
        created_at: 'now',
      },
    ];
    fetchMock.mockResolvedValueOnce(jsonResponse(feedback));

    const result = await listTrainingFeedback(1);

    expect(result).toEqual(feedback);
    expect(lastCall(fetchMock)[0]).toBe(
      '/admin/api/training/messages/1/feedback',
    );
  });

  it('listTrainingFeedback throws AdminApiError on a non-2xx response', async () => {
    fetchMock.mockResolvedValueOnce(
      jsonResponse({ error: 'server error' }, 500),
    );

    const error = await listTrainingFeedback(1).catch((e: unknown) => e);

    expect(error).toBeInstanceOf(AdminApiError);
    expect((error as AdminApiError).status).toBe(500);
  });

  it('throws AdminApiError with the parsed message on a non-2xx response', async () => {
    fetchMock.mockImplementation(() =>
      Promise.resolve(
        jsonResponse({ error: 'invalid or missing session cookie' }, 401),
      ),
    );

    const error = await getIngestConfig().catch((e: unknown) => e);

    expect(error).toBeInstanceOf(AdminApiError);
    expect((error as AdminApiError).status).toBe(401);
    expect((error as AdminApiError).message).toBe(
      'invalid or missing session cookie',
    );
  });

  it('falls back to a generic message when the error body is not JSON', async () => {
    fetchMock.mockResolvedValueOnce(new Response('not json', { status: 500 }));

    await expect(getIngestConfig()).rejects.toMatchObject({
      status: 500,
    });
  });

  it('login POSTs the username and password and sends credentials', async () => {
    fetchMock.mockResolvedValueOnce(jsonResponse({ status: 'logged_in' }));

    const result = await login('operator', 's3cret');

    expect(result).toEqual({ status: 'logged_in' });
    const [url, init] = lastCall(fetchMock);
    expect(url).toBe('/admin/api/auth/login');
    expect(init.method).toBe('POST');
    expect(init.credentials).toBe('include');
    expect(JSON.parse(init.body as string)).toEqual({
      username: 'operator',
      password: 's3cret',
    });
  });

  it('login throws AdminApiError on invalid credentials', async () => {
    fetchMock.mockResolvedValueOnce(
      jsonResponse({ error: 'invalid credentials' }, 401),
    );

    const error = await login('operator', 'wrong').catch((e: unknown) => e);

    expect(error).toBeInstanceOf(AdminApiError);
    expect((error as AdminApiError).status).toBe(401);
  });

  it('logout POSTs and sends credentials', async () => {
    fetchMock.mockResolvedValueOnce(jsonResponse({ status: 'logged_out' }));

    const result = await logout();

    expect(result).toEqual({ status: 'logged_out' });
    const [url, init] = lastCall(fetchMock);
    expect(url).toBe('/admin/api/auth/logout');
    expect(init.method).toBe('POST');
    expect(init.credentials).toBe('include');
  });
});
