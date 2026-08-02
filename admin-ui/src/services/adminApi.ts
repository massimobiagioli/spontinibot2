export class AdminApiError extends Error {
  readonly status: number;

  constructor(status: number, message: string) {
    super(message);
    this.name = 'AdminApiError';
    this.status = status;
  }
}

/**
 * Extracts a readable message from an error response body. The backend's own
 * errors are `{error: string}`, but a gateway/proxy in front of it (nginx,
 * a load balancer) can return a differently-shaped `{error: {message, ...}}`
 * on failures the backend never even saw — without this, that nested object
 * was passed straight to `AdminApiError`, which coerced it to the literal
 * string "[object Object]" via `Error`'s message-to-string conversion.
 */
function extractErrorMessage(body: unknown, fallback: string): string {
  if (body && typeof body === 'object' && 'error' in body) {
    const err = (body as { error?: unknown }).error;
    if (typeof err === 'string') {
      return err;
    }
    if (err && typeof err === 'object' && 'message' in err) {
      const nestedMessage = (err as { message?: unknown }).message;
      if (typeof nestedMessage === 'string') {
        return nestedMessage;
      }
    }
  }
  return fallback;
}

export interface IngestScheduleResponse {
  cron_expr: string;
  enabled: boolean;
  updated_at: string;
}

export interface RobotsBypassHostResponse {
  id: number;
  host: string;
  created_at: string;
}

export interface IngestSectionResponse {
  id: number;
  name: string;
  ordering: number;
  created_at: string;
}

export interface IngestSourceResponse {
  id: number;
  section_id: number;
  source_type: string;
  url: string;
  enabled: boolean;
  created_at: string;
  coming_soon: boolean;
}

export interface CurationSourceResponse {
  source_url: string;
  last_item_date: string;
  updated_at: string;
}

export interface IngestSectionWithSources extends IngestSectionResponse {
  sources: IngestSourceResponse[];
  curation_sources: CurationSourceResponse[];
}

export interface IngestConfigResponse {
  schedule: IngestScheduleResponse | null;
  sections: IngestSectionWithSources[];
}

export interface IngestedDocumentResponse {
  source_ref: string;
  source: string;
  chunk_count: number;
  created_at: string;
  summary: string | null;
}

export interface IngestRunResponse {
  id: number;
  status: string;
  requested_at: string;
}

export interface IngestManualResponse {
  section: string;
  src: string;
  window: string;
  status: string;
}

export interface UploadResponse {
  token: string;
  preview_url: string;
}

export interface UploadMetadataResponse {
  category: string | null;
  tags: string[] | null;
  trust_score: number | null;
}

export interface PreviewResponse {
  extracted_text: string;
  format: string;
  byte_size: number;
  section: string;
  filename: string;
  metadata: UploadMetadataResponse;
  chunk_count_estimate: number;
}

export interface ConfirmResponse {
  document_ids: number[];
  chunk_count: number;
}

export interface PersonaResponse {
  id: number;
  version: number;
  name: string;
  system_prompt: string;
  tone: string | null;
  fallback_message: string | null;
  is_active: boolean;
  created_at: string;
  created_by: string | null;
}

export interface CreatePersonaRequest {
  name: string;
  system_prompt: string;
  tone?: string;
  fallback_message?: string;
  activate: boolean;
}

export interface StatusResponse {
  status: string;
}

export interface TrainingSessionResponse {
  id: number;
  title: string;
  created_at: string;
  created_by: string | null;
  closed_at: string | null;
  notes: string | null;
}

export interface CreateSessionRequest {
  title: string;
  created_by?: string;
}

export interface ClosedResponse {
  closed: boolean;
}

export interface DeletedResponse {
  deleted: boolean;
}

export interface TrainingMessageSource {
  document_id: number;
  source_ref: string;
  source_url?: string;
}

export interface TrainingMessageResponse {
  id: number;
  session_id: number;
  question: string;
  answer: string;
  sources: TrainingMessageSource[];
  fell_back: boolean;
  created_at: string;
  expected_answer: string | null;
  execution_time_ms: number | null;
  source: string;
}

export interface AskTrainingMessageRequest {
  question: string;
  expected_answer?: string;
  /** A manually supplied answer; when set, the bot is not invoked. */
  answer?: string;
}

export interface TrainingFeedbackResponse {
  id: number;
  message_id: number;
  chunk_id: number | null;
  answer_span: string;
  sentiment: string;
  comment: string | null;
  created_at: string;
}

export interface CreateFeedbackRequest {
  message_id: number;
  chunk_id?: number;
  answer_span: string;
  sentiment: 'positive' | 'negative';
  comment?: string;
}

const LOGIN_PATH = '/admin/api/auth/login';

let unauthorizedHandler: (() => void) | null = null;

/**
 * Registers a callback invoked whenever any admin API call (other than the
 * login attempt itself) fails with 401 — the router wires this to redirect
 * to /login, without adminApi.ts depending on the router directly.
 */
export function onUnauthorized(handler: () => void): void {
  unauthorizedHandler = handler;
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, {
    ...init,
    credentials: 'include',
  });

  if (!response.ok) {
    const body: unknown = await response.json().catch(() => ({}));
    if (response.status === 401 && path !== LOGIN_PATH) {
      unauthorizedHandler?.();
    }
    throw new AdminApiError(
      response.status,
      extractErrorMessage(
        body,
        `request to ${path} failed with status ${response.status}`,
      ),
    );
  }

  return response.json() as Promise<T>;
}

function jsonRequest<T>(
  path: string,
  method: string,
  payload?: unknown,
): Promise<T> {
  const init: RequestInit =
    payload === undefined
      ? { method }
      : {
          method,
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(payload),
        };
  return request<T>(path, init);
}

export function login(
  username: string,
  password: string,
): Promise<StatusResponse> {
  return jsonRequest<StatusResponse>('/admin/api/auth/login', 'POST', {
    username,
    password,
  });
}

export function logout(): Promise<StatusResponse> {
  return request<StatusResponse>('/admin/api/auth/logout', { method: 'POST' });
}

export function getIngestConfig(): Promise<IngestConfigResponse> {
  return request<IngestConfigResponse>('/admin/api/ingest/config');
}

export function upsertSchedule(
  cronExpr: string,
  enabled: boolean,
): Promise<IngestScheduleResponse> {
  return jsonRequest<IngestScheduleResponse>(
    '/admin/api/ingest/config/schedule',
    'PUT',
    { cron_expr: cronExpr, enabled },
  );
}

export function listRobotsBypassHosts(): Promise<RobotsBypassHostResponse[]> {
  return request<RobotsBypassHostResponse[]>(
    '/admin/api/scraper/robots-bypass-hosts',
  );
}

export function replaceRobotsBypassHosts(
  hostsText: string,
): Promise<RobotsBypassHostResponse[]> {
  return jsonRequest<RobotsBypassHostResponse[]>(
    '/admin/api/scraper/robots-bypass-hosts',
    'PUT',
    { hosts_text: hostsText },
  );
}

export function createSection(
  name: string,
  ordering: number,
): Promise<IngestSectionResponse> {
  return jsonRequest<IngestSectionResponse>(
    '/admin/api/ingest/config/sections',
    'POST',
    { name, ordering },
  );
}

export async function deleteSection(id: number): Promise<boolean> {
  const result = await request<{ deleted: boolean }>(
    `/admin/api/ingest/config/sections/${id}`,
    { method: 'DELETE' },
  );
  return result.deleted;
}

export function listSectionDocuments(
  sectionId: number,
): Promise<IngestedDocumentResponse[]> {
  return request<IngestedDocumentResponse[]>(
    `/admin/api/ingest/config/sections/${sectionId}/documents`,
  );
}

export function createSource(
  sectionId: number,
  sourceType: string,
  url: string,
  enabled: boolean,
): Promise<IngestSourceResponse> {
  return jsonRequest<IngestSourceResponse>(
    `/admin/api/ingest/config/sources?section_id=${sectionId}`,
    'POST',
    { source_type: sourceType, url, enabled },
  );
}

export async function deleteSource(id: number): Promise<boolean> {
  const result = await request<{ deleted: boolean }>(
    `/admin/api/ingest/config/sources/${id}`,
    { method: 'DELETE' },
  );
  return result.deleted;
}

export function triggerIngestRun(): Promise<IngestRunResponse> {
  return request<IngestRunResponse>('/admin/api/ingest/run', {
    method: 'POST',
  });
}

export function getIngestRun(id: number): Promise<IngestRunResponse> {
  return request<IngestRunResponse>(`/admin/api/ingest/run/${id}`);
}

export function triggerManualIngest(
  section: string,
  src: string,
  window: string,
): Promise<IngestManualResponse> {
  return jsonRequest<IngestManualResponse>('/admin/api/ingest/manual', 'POST', {
    section,
    src,
    window,
  });
}

export async function uploadDocument(
  file: File,
  section: string,
): Promise<UploadResponse> {
  const form = new FormData();
  form.append('file', file, file.name);
  form.append('section', section);
  // Category, trust score, and tags are derived automatically by the
  // backend — the operator no longer supplies them.

  return request<UploadResponse>('/admin/api/upload', {
    method: 'POST',
    body: form,
  });
}

export function getUploadPreview(token: string): Promise<PreviewResponse> {
  return request<PreviewResponse>(`/admin/api/upload/preview/${token}`);
}

export function confirmUpload(token: string): Promise<ConfirmResponse> {
  return request<ConfirmResponse>(`/admin/api/upload/confirm/${token}`, {
    method: 'POST',
  });
}

export function getPersonaVersions(name: string): Promise<PersonaResponse[]> {
  return request<PersonaResponse[]>(
    `/admin/api/persona?name=${encodeURIComponent(name)}`,
  );
}

export function createPersona(
  payload: CreatePersonaRequest,
): Promise<PersonaResponse> {
  return jsonRequest<PersonaResponse>('/admin/api/persona', 'POST', payload);
}

export function activatePersona(id: number): Promise<StatusResponse> {
  return request<StatusResponse>(`/admin/api/persona/${id}/activate`, {
    method: 'POST',
  });
}

export function deletePersonaVersion(id: number): Promise<StatusResponse> {
  return request<StatusResponse>(`/admin/api/persona/${id}`, {
    method: 'DELETE',
  });
}

export function reloadPersona(): Promise<StatusResponse> {
  return request<StatusResponse>('/admin/api/persona/reload', {
    method: 'POST',
  });
}

export function createSession(
  title: string,
  createdBy?: string,
): Promise<TrainingSessionResponse> {
  const payload: CreateSessionRequest = createdBy
    ? { title, created_by: createdBy }
    : { title };
  return jsonRequest<TrainingSessionResponse>(
    '/admin/api/training/sessions',
    'POST',
    payload,
  );
}

export function listSessions(): Promise<TrainingSessionResponse[]> {
  return request<TrainingSessionResponse[]>('/admin/api/training/sessions');
}

export function getSession(id: number): Promise<TrainingSessionResponse> {
  return request<TrainingSessionResponse>(`/admin/api/training/sessions/${id}`);
}

export function closeSession(
  id: number,
  notes?: string,
): Promise<ClosedResponse> {
  const query = notes ? `?notes=${encodeURIComponent(notes)}` : '';
  return request<ClosedResponse>(
    `/admin/api/training/sessions/${id}/close${query}`,
    { method: 'POST' },
  );
}

export function deleteSession(id: number): Promise<DeletedResponse> {
  return request<DeletedResponse>(`/admin/api/training/sessions/${id}`, {
    method: 'DELETE',
  });
}

export function askTrainingMessage(
  sessionId: number,
  payload: AskTrainingMessageRequest,
): Promise<TrainingMessageResponse> {
  return jsonRequest<TrainingMessageResponse>(
    `/admin/api/training/sessions/${sessionId}/messages`,
    'POST',
    payload,
  );
}

export function listTrainingMessages(
  sessionId: number,
): Promise<TrainingMessageResponse[]> {
  return request<TrainingMessageResponse[]>(
    `/admin/api/training/sessions/${sessionId}/messages`,
  );
}

export function createTrainingFeedback(
  payload: CreateFeedbackRequest,
): Promise<TrainingFeedbackResponse> {
  return jsonRequest<TrainingFeedbackResponse>(
    '/admin/api/training/feedback',
    'POST',
    payload,
  );
}

export function listTrainingFeedback(
  messageId: number,
): Promise<TrainingFeedbackResponse[]> {
  return request<TrainingFeedbackResponse[]>(
    `/admin/api/training/messages/${messageId}/feedback`,
  );
}
