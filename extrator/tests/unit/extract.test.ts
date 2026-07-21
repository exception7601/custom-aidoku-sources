import { describe, expect, it } from 'vitest';

import { extractManifest } from '../../src/extract.js';
import { DEFAULT_SITE_URL, DEFAULT_SOURCE_ID } from '../../src/constants.js';
import { fixturePath } from '../helpers.js';

describe('manifest extraction', () => {
  it('builds a declarative manifest from a bundle file', async () => {
    const manifest = await extractManifest({
      sourceId: DEFAULT_SOURCE_ID,
      siteUrl: DEFAULT_SITE_URL,
      bundleFiles: [fixturePath('toonlivre-bundle-snippet.js')],
    });

    expect(manifest.sourceId).toBe(DEFAULT_SOURCE_ID);
    expect(manifest.bundle.discoveredFrom).toBe('file');
    expect(manifest.request.userAgent).toContain('Mozilla/5.0');
    expect(manifest.request.acceptLanguage).toBe('en-US,en;q=0.9,pt;q=0.8');
    expect(manifest.request.signatureHeader).toBe('x-toon-signature');
    expect(manifest.request.sessionCookie.name).toBe('toon_v');
    expect(manifest.decrypt.dataKeyHeader).toBe('x-toon-datakey');
    expect(manifest.decrypt.passphrase.prefix).toBe('Dealer-Critter-Catnip4');
  });

  it('extracts the seed-jwt strategy from the new bundle pattern', async () => {
    const manifest = await extractManifest({
      sourceId: DEFAULT_SOURCE_ID,
      siteUrl: DEFAULT_SITE_URL,
      bundleFiles: [fixturePath('toonlivre-bundle-seed-snippet.js')],
    });

    expect(manifest.request.signatureRules).toEqual([]);
    expect(manifest.request.signatureStrategy).toEqual({
      kind: 'seed-jwt',
      metaName: 't-seed',
      endpointPath: '/api/seed',
      tokenField: 'token',
    });
    expect(manifest.request.verifyHeader).toBeUndefined();
    expect(manifest.request.sessionCookie.mirrorsInto).toEqual([]);
    expect(manifest.decrypt.passphrase).toMatchObject({
      kind: 'utc-sha256-derived',
      prefix: 'Magnesium-Strike-Astonish3',
      salt: 'toonlivre.com::v8',
      suffix: 't8_4v2_b',
    });
  });
});
