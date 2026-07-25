export class ChatApiError extends Error {
  readonly status: number;

  constructor(status: number, message: string) {
    super(message);
    this.name = 'ChatApiError';
    this.status = status;
  }
}

export interface ChatSource {
  document_id: number;
  source_ref: string;
}

export interface ChatResponse {
  answer: string;
  sources: ChatSource[];
  fell_back: boolean;
}

export async function askChat(question: string): Promise<ChatResponse> {
  const response = await fetch('/chat', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ question }),
  });

  if (!response.ok) {
    const body = (await response.json().catch(() => ({}))) as {
      error?: string;
    };
    throw new ChatApiError(
      response.status,
      body.error ?? `request to /chat failed with status ${response.status}`,
    );
  }

  return response.json() as Promise<ChatResponse>;
}
