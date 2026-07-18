import { buildRequestContext, decryptPayload } from './runtime.js';
import type { ExtractedManifest } from './manifest.js';

export interface CanaryValidationResult {
  status: number;
  ok: boolean;
  dataKey?: string;
  pageCount: number;
  decrypted: Record<string, unknown>;
}

export async function validateManifestAgainstChapter(
  manifest: ExtractedManifest,
  chapterApiUrl: string
): Promise<CanaryValidationResult> {
  const requestContext = buildRequestContext(manifest, chapterApiUrl);
  requestContext.headers.set('origin', manifest.siteUrl);
  requestContext.headers.set('referer', manifest.siteUrl);
  requestContext.headers.set('cookie', requestContext.cookieHeader);

  const response = await fetch(chapterApiUrl, {
    headers: requestContext.headers,
  });
  const body = await response.text();
  if (!response.ok) {
    throw new Error(`Canary request failed with ${response.status}: ${body.slice(0, 300)}`);
  }

  const dataKey = response.headers.get(manifest.decrypt.dataKeyHeader) ?? undefined;
  const decryptedText = decryptPayload(manifest, body, dataKey);
  const decrypted = JSON.parse(decryptedText) as Record<string, unknown>;
  const pages = Array.isArray(decrypted.pages) ? decrypted.pages : [];

  return {
    status: response.status,
    ok: response.ok,
    dataKey,
    pageCount: pages.length,
    decrypted,
  };
}
