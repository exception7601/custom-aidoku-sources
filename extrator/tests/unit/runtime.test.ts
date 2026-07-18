import CryptoJS from 'crypto-js';
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';

import { DEFAULT_SITE_URL, DEFAULT_SOURCE_ID } from '../../src/constants.js';
import { extractManifest } from '../../src/extract.js';
import { buildRequestContext, decryptPayload, derivePassphrase } from '../../src/runtime.js';
import type { ExtractedManifest } from '../../src/manifest.js';
import { fixturePath } from '../helpers.js';

describe('runtime helpers', () => {
  let manifest: ExtractedManifest;

  beforeAll(async () => {
    manifest = await extractManifest({
      sourceId: DEFAULT_SOURCE_ID,
      siteUrl: DEFAULT_SITE_URL,
      bundleFiles: [fixturePath('toonlivre-bundle-snippet.js')],
    });
  });

  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(Date.UTC(2026, 6, 18, 12, 0, 0)));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('builds request headers and a mirrored cookie value', () => {
    const chapterContext = buildRequestContext(
      manifest,
      'https://toonlivre.net/api/mangas/obra/chapters/cap-1'
    );
    const listContext = buildRequestContext(manifest, 'https://toonlivre.net/api/mangas/releases');

    expect(chapterContext.headers.get('user-agent')).toBe(manifest.request.userAgent);
    expect(chapterContext.headers.get('accept-language')).toBe(manifest.request.acceptLanguage);
    expect(chapterContext.headers.get('x-toon-signature')).toBe('t8v_authX9');
    expect(listContext.headers.get('x-toon-signature')).toBe('t8v_decoy9');
    expect(chapterContext.headers.get('x-toon-verify')).toBe(chapterContext.sessionValue);
    expect(chapterContext.cookieHeader.startsWith('toon_v=')).toBe(true);
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
});
