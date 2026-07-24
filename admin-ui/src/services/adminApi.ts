const ADMIN_KEY_HEADER = 'X-Admin-Key';

function adminKey(): string {
  return import.meta.env.VITE_ADMIN_API_KEY ?? 'dev-key';
}

export class AdminApiError extends Error {
  readonly status: number;

  constructor(status: number, message: string) {
    super(message);
    this.name = 'AdminApiError';
    this.status = status;
  }
}

export interface IngestScheduleResponse {
  cron_expr: string;
  enabled: boolean;
  updated_at: string;
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

export interface IngestSectionWithSources extends IngestSectionResponse {
  sources: IngestSourceResponse[];
}

export interface IngestConfigResponse {
  schedule: IngestScheduleResponse | null;
  sections: IngestSectionWithSources[];
}

export interface IngestRunResponse {
  id: number;
  status: string;
  requested_at: string;
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

export interface UploadMetadataInput {
  category?: string;
  tags?: string[];
  trustScore?: number;
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
}

export interface CreateSessionRequest {
  title: string;
  created_by?: string;
}

export interface ClosedResponse {
  closed: boolean;
}

export interface TrainingMessageSource {
  document_id: number;
  source_ref: string;
}

export interface TrainingMessageResponse {
  id: number;
  session_id: number;
  question: string;
  answer: string;
  sources: TrainingMessageSource[];
  fell_back: boolean;
  created_at: string;
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

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, {
    ...init,
    headers: {
      [ADMIN_KEY_HEADER]: adminKey(),
      ...(init?.headers ?? {}),
    },
  });

  if (!response.ok) {
    const body = (await response.json().catch(() => ({}))) as {
      error?: string;
    };
    throw new AdminApiError(
      response.status,
      body.error ?? `request to ${path} failed with status ${response.status}`,
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

export async function uploadDocument(
  file: File,
  section: string,
  metadata: UploadMetadataInput,
): Promise<UploadResponse> {
  const form = new FormData();
  form.append('file', file, file.name);
  form.append('section', section);
  if (metadata.category) form.append('category', metadata.category);
  if (metadata.tags && metadata.tags.length > 0) {
    form.append('tags', metadata.tags.join(','));
  }
  if (metadata.trustScore !== undefined) {
    form.append('trust_score', String(metadata.trustScore));
  }

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

export function closeSession(id: number): Promise<ClosedResponse> {
  return request<ClosedResponse>(`/admin/api/training/sessions/${id}/close`, {
    method: 'POST',
  });
}

export function askTrainingMessage(
  sessionId: number,
  question: string,
): Promise<TrainingMessageResponse> {
  return jsonRequest<TrainingMessageResponse>(
    `/admin/api/training/sessions/${sessionId}/messages`,
    'POST',
    { question },
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
