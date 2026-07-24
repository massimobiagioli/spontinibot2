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
