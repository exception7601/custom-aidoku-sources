import CryptoJS from 'crypto-js';

import { md5, sha256 } from './http.js';
import type {
  DynamicSignatureStrategy,
  ExtractedManifest,
  SignatureRule,
} from './manifest.js';

export interface RequestContext {
  headers: Headers;
  cookieHeader: string;
  sessionValue: string;
}

export function buildRequestContext(manifest: ExtractedManifest, url: string): RequestContext {
  const sessionValue = generateSessionCookieValue(manifest);
  const signatureValue = buildSignatureValue(manifest, url);
  const headers = new Headers({
    'user-agent': manifest.request.userAgent,
    'accept-language': manifest.request.acceptLanguage,
    [manifest.request.signatureHeader]: signatureValue,
    [manifest.request.verifyHeader]: sessionValue,
    accept: 'application/json, text/plain, */*',
  });

  return {
    headers,
    cookieHeader: `${manifest.request.sessionCookie.name}=${sessionValue}`,
    sessionValue,
  };
}

export function generateSessionCookieValue(manifest: ExtractedManifest): string {
  return manifest.request.sessionCookie.generator.segments
    .map((segment) => Math.random().toString(segment.radix).substring(segment.start, segment.end))
    .join('');
}

export function buildSignatureValue(manifest: ExtractedManifest, url: string): string {
  if (manifest.request.signatureStrategy) {
    return deriveDynamicSignatureValue(manifest.request.signatureStrategy, url);
  }

  return selectSignatureValue(manifest.request.signatureRules, url);
}

export function selectSignatureValue(rules: SignatureRule[], url: string): string {
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
  strategy: DynamicSignatureStrategy,
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
  const seed =
    formattedDate + manifest.decrypt.passphrase.salt + manifest.decrypt.passphrase.suffix;
  const digest = md5(seed).slice(
    manifest.decrypt.passphrase.digestSlice.start,
    manifest.decrypt.passphrase.digestSlice.end
  );

  return manifest.decrypt.passphrase.prefix + digest;
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

export function selectEncryptedField(
  parsed: Record<string, unknown>,
  dataKey?: string
): string | undefined {
  if (dataKey && typeof parsed[dataKey] === 'string') {
    return parsed[dataKey] as string;
  }

  return Object.values(parsed).find((value): value is string => typeof value === 'string');
}
