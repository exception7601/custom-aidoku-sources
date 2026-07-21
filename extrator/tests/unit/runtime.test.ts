import CryptoJS from 'crypto-js';
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';

import { DEFAULT_SITE_URL, DEFAULT_SOURCE_ID } from '../../src/constants.js';
import { extractManifest } from '../../src/extract.js';
import {
  buildRequestContext,
  decryptPayload,
  deriveDynamicSignatureValue,
  derivePassphrase,
} from '../../src/runtime.js';
import type { ExtractedManifest } from '../../src/manifest.js';
import { fixturePath } from '../helpers.js';

describe('runtime helpers', () => {
  let manifest: ExtractedManifest;
  let seedManifest: ExtractedManifest;

  beforeAll(async () => {
    manifest = await extractManifest({
      sourceId: DEFAULT_SOURCE_ID,
      siteUrl: DEFAULT_SITE_URL,
      bundleFiles: [fixturePath('toonlivre-bundle-snippet.js')],
    });
    seedManifest = await extractManifest({
      sourceId: DEFAULT_SOURCE_ID,
      siteUrl: DEFAULT_SITE_URL,
      entryUrl: 'https://toonlivre.net/',
      bundleFiles: [fixturePath('toonlivre-bundle-seed-snippet.js')],
    });
  });

  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(Date.UTC(2026, 6, 18, 12, 0, 0)));
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.useRealTimers();
  });

  it('builds request headers and a mirrored cookie value', async () => {
    const chapterContext = await buildRequestContext(
      manifest,
      'https://toonlivre.net/api/mangas/obra/chapters/cap-1'
    );
    const listContext = await buildRequestContext(
      manifest,
      'https://toonlivre.net/api/mangas/releases'
    );

    expect(chapterContext.headers.get('user-agent')).toBe(manifest.request.userAgent);
    expect(chapterContext.headers.get('accept-language')).toBe(manifest.request.acceptLanguage);
    expect(chapterContext.headers.get('x-toon-signature')).toBe('t8v_authX9');
    expect(listContext.headers.get('x-toon-signature')).toBe('t8v_decoy9');
    expect(chapterContext.headers.get('x-toon-verify')).toBe(chapterContext.sessionValue);
    expect(chapterContext.cookieHeader.startsWith('toon_v=')).toBe(true);
  });

  it('derives a dynamic signature when the manifest provides a strategy', async () => {
    const dynamicManifest: ExtractedManifest = {
      ...manifest,
      request: {
        ...manifest.request,
        signatureRules: [],
        signatureStrategy: {
          kind: 'time-sha256-base64',
          timestampDivisor: 30000,
          salt: 'My4xNDE=_1388',
          routeSelector: {
            whenUrlContains: '/chapters',
            whenMatched: 'chapters',
            otherwise: 'other',
          },
        },
      },
    };
    const strategy = dynamicManifest.request.signatureStrategy;
    if (!strategy || strategy.kind !== 'time-sha256-base64') {
      throw new Error('Expected a time-sha256-base64 strategy.');
    }

    const chapterContext = await buildRequestContext(
      dynamicManifest,
      'https://toonlivre.net/api/mangas/obra/chapters/cap-1'
    );
    const listContext = await buildRequestContext(
      dynamicManifest,
      'https://toonlivre.net/api/mangas/releases'
    );

    expect(chapterContext.headers.get('x-toon-signature')).toBe(
      deriveDynamicSignatureValue(strategy, 'https://toonlivre.net/api/mangas/obra/chapters/cap-1')
    );
    expect(listContext.headers.get('x-toon-signature')).toBe(
      deriveDynamicSignatureValue(strategy, 'https://toonlivre.net/api/mangas/releases')
    );
    expect(chapterContext.headers.get('x-toon-signature')).not.toBe(
      listContext.headers.get('x-toon-signature')
    );
  });

  it('builds a seed-backed signature from the HTML meta token', async () => {
    const payload = Buffer.from(
      JSON.stringify({ exp: Math.floor(Date.now() / 1000) + 600 })
    ).toString('base64url');
    const token = `header.${payload}.signature`;

    vi.stubGlobal(
      'fetch',
      vi.fn(async (input: RequestInfo | URL) => {
        const url = typeof input === 'string' ? input : input.toString();

        if (url === 'https://toonlivre.net/') {
          return new Response(
            `<html><head><meta name="t-seed" content="${token}"></head><body></body></html>`,
            {
              status: 200,
              headers: {
                'content-type': 'text/html; charset=utf-8',
              },
            }
          );
        }

        throw new Error(`Unexpected fetch: ${url}`);
      })
    );

    const requestContext = await buildRequestContext(
      seedManifest,
      'https://toonlivre.net/api/mangas/obra/chapters/cap-1'
    );

    expect(requestContext.headers.get('x-toon-signature')).toBe(token);
    expect(requestContext.headers.get('x-toon-verify')).toBeNull();
    expect(requestContext.cookieHeader.startsWith('toon_v=')).toBe(true);
  });

  it('derives the passphrase and decrypts a response payload', () => {
    const payload = {
      id: 'cap-1',
      pages: ['https://cdn.toonlivre.net/obras/obra-1/1.webp'],
    };
    const passphrase = derivePassphrase(manifest);
    const encrypted = CryptoJS.Rabbit.encrypt(JSON.stringify(payload), passphrase).toString();
    const decrypted = decryptPayload(manifest, JSON.stringify({ demo: encrypted }), 'demo');

    expect(passphrase.startsWith('Dealer-Critter-Catnip4')).toBe(true);
    expect(JSON.parse(decrypted)).toEqual(payload);
  });

  it('derives the SHA-256 passphrase for the new manifest pattern', () => {
    const passphrase = derivePassphrase(seedManifest);

    expect(passphrase.startsWith('Magnesium-Strike-Astonish3')).toBe(true);
    expect(passphrase).toHaveLength('Magnesium-Strike-Astonish3'.length + 8);
  });
});
