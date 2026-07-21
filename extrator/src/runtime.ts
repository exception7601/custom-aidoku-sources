import { load } from 'cheerio';
import CryptoJS from 'crypto-js';

import { fetchText, md5, sha256 } from './http.js';
import type {
  DynamicSignatureStrategy,
  ExtractedManifest,
  PassphraseStrategy,
  SignatureRule,
} from './manifest.js';

const SEED_TOKEN_EXPIRY_MARGIN_MS = 120_000;
const SEED_TOKEN_FALLBACK_TTL_MS = 25 * 60_000;

const seedTokenCache = new Map<
  string,
  {
    token: string;
    expiresAt: number;
  }
>();

export interface RequestContext {
  headers: Headers;
  cookieHeader: string;
  sessionValue: string;
}

export async function buildRequestContext(
  manifest: ExtractedManifest,
  url: string
): Promise<RequestContext> {
  const sessionValue = generateSessionCookieValue(manifest);
  const signatureValue = await buildSignatureValue(manifest, url, sessionValue);
  const headers = new Headers({
    'user-agent': manifest.request.userAgent,
    'accept-language': manifest.request.acceptLanguage,
    [manifest.request.signatureHeader]: signatureValue,
    accept: 'application/json, text/plain, */*',
  });

  if (manifest.request.verifyHeader) {
    headers.set(manifest.request.verifyHeader, sessionValue);
  }

  for (const headerName of manifest.request.sessionCookie.mirrorsInto) {
    headers.set(headerName, sessionValue);
  }

  return {
    headers,
    cookieHeader: `${manifest.request.sessionCookie.name}=${sessionValue}`,
    sessionValue,
  };
}

function generateSessionCookieValue(manifest: ExtractedManifest): string {
  return manifest.request.sessionCookie.generator.segments
    .map((segment) => Math.random().toString(segment.radix).substring(segment.start, segment.end))
    .join('');
}

async function buildSignatureValue(
  manifest: ExtractedManifest,
  url: string,
  sessionValue: string
): Promise<string> {
  if (manifest.request.signatureStrategy) {
    return resolveSignatureStrategyValue(manifest, manifest.request.signatureStrategy, url, sessionValue);
  }

  return selectSignatureValue(manifest.request.signatureRules, url);
}

function selectSignatureValue(rules: SignatureRule[], url: string): string {
  for (const rule of rules) {
    if (rule.default === true) {
      continue;
    }

    if (rule.when?.urlContains && url.includes(rule.when.urlContains)) {
      return rule.value;
    }
  }

  const defaultRule = rules.find((rule) => rule.default === true);
  if (!defaultRule) {
    throw new Error('No default signature rule found.');
  }

  return defaultRule.value;
}

export function deriveDynamicSignatureValue(
  strategy: Extract<DynamicSignatureStrategy, { kind: 'time-sha256-base64' }>,
  url: string,
  now = Date.now()
): string {
  const timestamp = Math.floor(now / strategy.timestampDivisor);
  const routeKind = url.includes(strategy.routeSelector.whenUrlContains)
    ? strategy.routeSelector.whenMatched
    : strategy.routeSelector.otherwise;
  const payload = `${timestamp}:${routeKind}:${strategy.salt}`;
  const digest = sha256(payload);

  return Buffer.from(`${timestamp}:${digest}`).toString('base64');
}

export function derivePassphrase(manifest: ExtractedManifest, date = new Date()): string {
  const year = date.getUTCFullYear();
  const month = String(date.getUTCMonth() + 1).padStart(2, '0');
  const day = String(date.getUTCDate()).padStart(2, '0');
  const formattedDate = `${year}-${month}-${day}`;

  return buildDerivedPassphrase(manifest.decrypt.passphrase, formattedDate);
}

export function decryptPayload(
  manifest: ExtractedManifest,
  responseBody: string,
  dataKey?: string
): string {
  const parsed = JSON.parse(responseBody) as Record<string, unknown>;
  const encryptedValue = selectEncryptedField(parsed, dataKey);
  if (!encryptedValue) {
    throw new Error('Encrypted payload field not found.');
  }

  const passphrase = derivePassphrase(manifest);
  const decrypted = CryptoJS.Rabbit.decrypt(encryptedValue, passphrase).toString(CryptoJS.enc.Utf8);
  if (!decrypted) {
    throw new Error('Failed to decrypt Rabbit payload.');
  }

  return decrypted;
}

function selectEncryptedField(
  parsed: Record<string, unknown>,
  dataKey?: string
): string | undefined {
  if (dataKey && typeof parsed[dataKey] === 'string') {
    return parsed[dataKey] as string;
  }

  return Object.values(parsed).find((value): value is string => typeof value === 'string');
}

async function resolveSignatureStrategyValue(
  manifest: ExtractedManifest,
  strategy: DynamicSignatureStrategy,
  url: string,
  sessionValue: string
): Promise<string> {
  switch (strategy.kind) {
    case 'time-sha256-base64':
      return deriveDynamicSignatureValue(strategy, url);
    case 'seed-jwt':
      return resolveSeedJwtSignatureValue(manifest, strategy, sessionValue);
  }
}

async function resolveSeedJwtSignatureValue(
  manifest: ExtractedManifest,
  strategy: Extract<DynamicSignatureStrategy, { kind: 'seed-jwt' }>,
  sessionValue: string
): Promise<string> {
  const cacheKey = new URL(manifest.siteUrl).origin;
  const cached = getCachedSeedToken(cacheKey);
  if (cached) {
    return cached;
  }

  const cookieHeader = `${manifest.request.sessionCookie.name}=${sessionValue}`;
  const entryUrl = manifest.entryUrl ?? manifest.siteUrl;

  try {
    const htmlResponse = await fetchText(entryUrl, {
      headers: {
        accept: 'text/html,application/xhtml+xml',
        cookie: cookieHeader,
      },
    });
    const seedFromHtml = extractSeedTokenFromHtml(htmlResponse.body, strategy.metaName);

    if (seedFromHtml && tokenIsFresh(seedFromHtml)) {
      cacheSeedToken(cacheKey, seedFromHtml);
      return seedFromHtml;
    }
  } catch {
    // Fall back to the JSON seed endpoint.
  }

  const seedUrl = new URL(strategy.endpointPath, manifest.siteUrl).toString();
  const seedResponse = await fetchText(seedUrl, {
    headers: {
      accept: 'application/json',
      cookie: cookieHeader,
    },
  });

  let parsed: Record<string, unknown>;
  try {
    parsed = JSON.parse(seedResponse.body) as Record<string, unknown>;
  } catch (error: unknown) {
    throw new Error(
      `Seed endpoint returned invalid JSON (${seedResponse.url}): ${toErrorMessage(error)}`
    );
  }

  const token = parsed[strategy.tokenField];
  if (typeof token !== 'string' || token.trim().length === 0) {
    throw new Error(
      `Seed endpoint response did not include a non-empty \
\`${strategy.tokenField}\` token (${seedResponse.url}).`
    );
  }

  cacheSeedToken(cacheKey, token);
  return token;
}

function buildDerivedPassphrase(strategy: PassphraseStrategy, formattedDate: string): string {
  const seed = formattedDate + strategy.salt + strategy.suffix;

  switch (strategy.kind) {
    case 'utc-md5-derived': {
      const digest = md5(seed).slice(strategy.digestSlice.start, strategy.digestSlice.end);
      return strategy.prefix + digest;
    }
    case 'utc-sha256-derived': {
      const digest = sha256(seed).slice(strategy.digestSlice.start, strategy.digestSlice.end);
      return strategy.prefix + digest;
    }
  }
}

function extractSeedTokenFromHtml(html: string, metaName: string): string | undefined {
  const $ = load(html);
  const value = $(`meta[name="${metaName}"]`).attr('content')?.trim();

  return value ? value : undefined;
}

function getCachedSeedToken(cacheKey: string): string | undefined {
  const cached = seedTokenCache.get(cacheKey);
  if (!cached) {
    return undefined;
  }

  if (cached.expiresAt <= Date.now() + SEED_TOKEN_EXPIRY_MARGIN_MS) {
    seedTokenCache.delete(cacheKey);
    return undefined;
  }

  return cached.token;
}

function cacheSeedToken(cacheKey: string, token: string): void {
  const expiresAt = decodeJwtExpiry(token) ?? Date.now() + SEED_TOKEN_FALLBACK_TTL_MS;

  seedTokenCache.set(cacheKey, {
    token,
    expiresAt,
  });
}

function tokenIsFresh(token: string): boolean {
  const expiresAt = decodeJwtExpiry(token);

  return expiresAt !== undefined && expiresAt > Date.now() + SEED_TOKEN_EXPIRY_MARGIN_MS;
}

function decodeJwtExpiry(token: string): number | undefined {
  const segments = token.split('.');
  if (segments.length !== 3) {
    return undefined;
  }

  try {
    const payload = JSON.parse(decodeBase64Url(segments[1]));
    const exp = payload?.exp;

    return typeof exp === 'number' && Number.isFinite(exp) ? exp * 1_000 : undefined;
  } catch {
    return undefined;
  }
}

function decodeBase64Url(input: string | undefined): string {
  if (!input) {
    throw new Error('Missing base64url segment.');
  }

  const normalized = input.replace(/-/g, '+').replace(/_/g, '/');
  const paddingLength = (4 - (normalized.length % 4)) % 4;

  return Buffer.from(`${normalized}${'='.repeat(paddingLength)}`, 'base64').toString('utf8');
}

function toErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
