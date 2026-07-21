import { buildRequestContext, decryptPayload } from './runtime.js';
import type { ExtractedManifest } from './manifest.js';

const FAILURE_BODY_LIMIT = 220;
const INTERESTING_RESPONSE_HEADERS = [
  'content-type',
  'cf-ray',
  'retry-after',
  'ratelimit-remaining',
  'ratelimit-reset',
] as const;

export interface CanaryValidationResult {
  status: number;
  ok: boolean;
  dataKey?: string;
  pageCount: number;
  decrypted: Record<string, unknown>;
}

interface CanaryRequestFailureArgs {
  manifest: ExtractedManifest;
  chapterApiUrl: string;
  status: number;
  body: string;
  responseHeaders: Headers;
  signatureValue: string;
}

interface CanaryPayloadFailureArgs {
  manifest: ExtractedManifest;
  chapterApiUrl: string;
  dataKey?: string;
  body: string;
  causeMessage: string;
  stage: 'decrypt' | 'parse';
}

export async function validateManifestAgainstChapter(
  manifest: ExtractedManifest,
  chapterApiUrl: string
): Promise<CanaryValidationResult> {
  const requestContext = await buildRequestContext(manifest, chapterApiUrl);
  requestContext.headers.set('origin', manifest.siteUrl);
  requestContext.headers.set('referer', manifest.siteUrl);
  requestContext.headers.set('cookie', requestContext.cookieHeader);

  const signatureValue =
    requestContext.headers.get(manifest.request.signatureHeader) ?? 'missing-signature';
  const response = await fetch(chapterApiUrl, {
    headers: requestContext.headers,
  });
  const body = await response.text();
  if (!response.ok) {
    throw new Error(
      buildCanaryRequestFailureMessage({
        manifest,
        chapterApiUrl,
        status: response.status,
        body,
        responseHeaders: response.headers,
        signatureValue,
      })
    );
  }

  const dataKey = response.headers.get(manifest.decrypt.dataKeyHeader) ?? undefined;

  let decryptedText: string;
  try {
    decryptedText = decryptPayload(manifest, body, dataKey);
  } catch (error: unknown) {
    throw new Error(
      buildCanaryPayloadFailureMessage({
        manifest,
        chapterApiUrl,
        dataKey,
        body,
        causeMessage: toErrorMessage(error),
        stage: 'decrypt',
      }),
      { cause: toErrorCause(error) }
    );
  }

  let decrypted: Record<string, unknown>;
  try {
    decrypted = JSON.parse(decryptedText) as Record<string, unknown>;
  } catch (error: unknown) {
    throw new Error(
      buildCanaryPayloadFailureMessage({
        manifest,
        chapterApiUrl,
        dataKey,
        body: decryptedText,
        causeMessage: toErrorMessage(error),
        stage: 'parse',
      }),
      { cause: toErrorCause(error) }
    );
  }

  const pages = Array.isArray(decrypted.pages) ? decrypted.pages : [];

  return {
    status: response.status,
    ok: response.ok,
    dataKey,
    pageCount: pages.length,
    decrypted,
  };
}

export function buildCanaryRequestFailureMessage({
  manifest,
  chapterApiUrl,
  status,
  body,
  responseHeaders,
  signatureValue,
}: CanaryRequestFailureArgs): string {
  const lines = [
    'ToonLivre canary request failed.',
    `URL: ${chapterApiUrl}`,
    `Status: ${status} ${describeHttpStatus(status)}`,
    `Bundle: ${manifest.bundle.url ?? manifest.entryUrl ?? manifest.siteUrl}`,
    `Request signature: ${manifest.request.signatureHeader}=${signatureValue}`,
    `Token mirror: ${describeTokenMirror(manifest)}`,
  ];
  const responseHeaderSummary = summarizeHeaders(responseHeaders);
  const apiError = extractApiError(body);
  const hint = buildRequestHint(status, responseHeaders, body, manifest);
  const bodySnippet = summarizeBody(body);

  if (responseHeaderSummary) {
    lines.push(`Response headers: ${responseHeaderSummary}`);
  }

  if (apiError) {
    lines.push(`Response error: ${apiError}`);
  }

  if (hint) {
    lines.push(`Hint: ${hint}`);
  }

  if (bodySnippet) {
    lines.push(`Body: ${bodySnippet}`);
  }

  return lines.join('\n');
}

export function buildCanaryPayloadFailureMessage({
  manifest,
  chapterApiUrl,
  dataKey,
  body,
  causeMessage,
  stage,
}: CanaryPayloadFailureArgs): string {
  const lines = [
    stage === 'decrypt'
      ? 'ToonLivre canary response could not be decrypted.'
      : 'ToonLivre canary response decrypted, but the JSON payload could not be parsed.',
    `URL: ${chapterApiUrl}`,
    `Bundle: ${manifest.bundle.url ?? manifest.entryUrl ?? manifest.siteUrl}`,
    `Data key: ${manifest.decrypt.dataKeyHeader}=${dataKey ?? 'missing'}`,
    `Algorithm: ${manifest.decrypt.algorithm}`,
    `Cause: ${causeMessage}`,
    stage === 'decrypt'
      ? 'Hint: a receita do manifesto para data key, algoritmo ou passphrase pode ter ficado desatualizada.'
      : 'Hint: a descriptografia funcionou, mas o formato JSON retornado mudou e o manifest/runtime precisa ser revisto.',
  ];
  const bodySnippet = summarizeBody(body);

  if (bodySnippet) {
    lines.push(`${stage === 'decrypt' ? 'Body' : 'Payload'}: ${bodySnippet}`);
  }

  return lines.join('\n');
}

function buildRequestHint(
  status: number,
  responseHeaders: Headers,
  body: string,
  manifest: ExtractedManifest
): string | undefined {
  if (status === 403) {
    const pieces = [`\`${manifest.request.signatureHeader}\``];

    if (manifest.request.verifyHeader) {
      pieces.push(`\`${manifest.request.verifyHeader}\``);
    }

    pieces.push(`cookie \`${manifest.request.sessionCookie.name}\``);

    return `O endpoint de capítulo rejeitou a assinatura do manifesto ou a semente/token dinâmico. Confira ${pieces.join(', ')}.`;
  }

  if (status === 429) {
    const waitSeconds =
      responseHeaders.get('retry-after') ?? responseHeaders.get('ratelimit-reset');

    return waitSeconds
      ? `O site limitou as requisições. Aguarde ${waitSeconds} segundo(s) antes de tentar novamente.`
      : 'O site limitou as requisições. Aguarde alguns instantes antes de tentar novamente.';
  }

  if (isHtmlLikeResponse(responseHeaders, body)) {
    return (
      'O site respondeu HTML/Cloudflare em vez de JSON. Pode ser um bloqueio temporário, ' +
      'um desafio anti-bot ou uma URL de bundle incorreta.'
    );
  }

  if (status >= 500) {
    return 'O ToonLivre respondeu com erro interno. Tente novamente mais tarde.';
  }

  return undefined;
}

function summarizeHeaders(headers: Headers): string {
  return INTERESTING_RESPONSE_HEADERS.map((headerName) => {
    const value = headers.get(headerName);

    return value ? `${headerName}=${value}` : null;
  })
    .filter((entry): entry is string => entry !== null)
    .join(', ');
}

function extractApiError(body: string): string | undefined {
  try {
    const parsed = JSON.parse(body) as Record<string, unknown>;

    if (typeof parsed.error === 'string' && parsed.error.trim().length > 0) {
      return parsed.error;
    }

    if (typeof parsed.message === 'string' && parsed.message.trim().length > 0) {
      return parsed.message;
    }

    return undefined;
  } catch {
    return undefined;
  }
}

function isHtmlLikeResponse(headers: Headers, body: string): boolean {
  const contentType = headers.get('content-type')?.toLowerCase() ?? '';
  const normalizedBody = body.trim().toLowerCase();

  return (
    contentType.includes('text/html') ||
    normalizedBody.startsWith('<!doctype html') ||
    normalizedBody.startsWith('<html') ||
    normalizedBody.includes('cloudflare') ||
    normalizedBody.includes('cdn-cgi')
  );
}

function summarizeBody(body: string): string {
  return body.replace(/\s+/g, ' ').trim().slice(0, FAILURE_BODY_LIMIT);
}

function describeHttpStatus(status: number): string {
  switch (status) {
    case 400:
      return 'Bad Request';
    case 401:
      return 'Unauthorized';
    case 403:
      return 'Forbidden';
    case 404:
      return 'Not Found';
    case 429:
      return 'Too Many Requests';
    case 500:
      return 'Internal Server Error';
    case 502:
      return 'Bad Gateway';
    case 503:
      return 'Service Unavailable';
    case 504:
      return 'Gateway Timeout';
    default:
      return 'Unexpected Response';
  }
}

function describeTokenMirror(manifest: ExtractedManifest): string {
  const parts: string[] = [];

  if (manifest.request.verifyHeader) {
    parts.push(manifest.request.verifyHeader);
  }

  for (const headerName of manifest.request.sessionCookie.mirrorsInto) {
    if (!parts.includes(headerName)) {
      parts.push(headerName);
    }
  }

  parts.push(`cookie ${manifest.request.sessionCookie.name}`);

  return parts.join(' + ');
}

function toErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function toErrorCause(error: unknown): Error {
  return error instanceof Error ? error : new Error(String(error));
}
