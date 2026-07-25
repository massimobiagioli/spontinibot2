import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { askChat, ChatApiError } from '../chatApi';

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

describe('chatApi', () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    fetchMock = vi.fn();
    vi.stubGlobal('fetch', fetchMock);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('posts the question as JSON to /chat and returns the parsed answer', async () => {
    const response = {
      answer: 'Lo sportello apre alle 9:00',
      sources: [{ document_id: 1, source_ref: 'orari.md' }],
      fell_back: false,
    };
    fetchMock.mockResolvedValueOnce(jsonResponse(response));

    const result = await askChat('A che ore apre lo sportello?');

    expect(result).toEqual(response);
    const [url, init] = lastCall(fetchMock);
    expect(url).toBe('/chat');
    expect(init.method).toBe('POST');
    expect((init.headers as Record<string, string>)['Content-Type']).toBe(
      'application/json',
    );
    expect(JSON.parse(init.body as string)).toEqual({
      question: 'A che ore apre lo sportello?',
    });
  });

  it('does not send an admin-key header — /chat is public', async () => {
    fetchMock.mockResolvedValueOnce(
      jsonResponse({ answer: '', sources: [], fell_back: true }),
    );

    await askChat('domanda');

    const [, init] = lastCall(fetchMock);
    expect(
      (init.headers as Record<string, string>)['X-Admin-Key'],
    ).toBeUndefined();
  });

  it('throws a ChatApiError with the parsed error message on a non-2xx response', async () => {
    fetchMock.mockResolvedValueOnce(
      jsonResponse({ error: 'no active persona configured' }, 503),
    );

    await expect(askChat('domanda')).rejects.toMatchObject({
      name: 'ChatApiError',
      status: 503,
      message: 'no active persona configured',
    });
  });

  it('throws a ChatApiError with a generic message when the error body is not JSON', async () => {
    fetchMock.mockResolvedValue(new Response('not json', { status: 502 }));

    await expect(askChat('domanda')).rejects.toThrow(ChatApiError);
    await expect(askChat('domanda')).rejects.toMatchObject({
      status: 502,
      message: 'request to /chat failed with status 502',
    });
  });
});
