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
});
