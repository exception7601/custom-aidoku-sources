import { describe, expect, it } from 'vitest';

import {
  DEFAULT_CANARY_CHAPTER_URL,
  DEFAULT_SITE_URL,
  DEFAULT_SOURCE_ID,
} from '../../src/constants.js';
import { extractManifest } from '../../src/extract.js';
import { validateManifestAgainstChapter } from '../../src/validate.js';

const liveDescribe = process.env.RUN_LIVE_TESTS === '1' ? describe : describe.skip;
const bundleUrl = process.env.TOONLIVRE_BUNDLE_URL;
const bundleInput = bundleUrl ?? DEFAULT_SITE_URL;
const canaryChapterUrl = process.env.TOONLIVRE_CANARY_CHAPTER_URL ?? DEFAULT_CANARY_CHAPTER_URL;

liveDescribe('live ToonLivre extraction', () => {
  it('extracts a manifest from the live bundle and validates chapter access', async () => {
    const manifest = await extractManifest({
      sourceId: DEFAULT_SOURCE_ID,
      siteUrl: DEFAULT_SITE_URL,
      bundleUrls: [bundleInput],
    }).catch((error: unknown) => {
      if (bundleUrl) {
        throw error;
      }

      const message = error instanceof Error ? error.message : String(error);
      throw new Error(
        'Base URL discovery failed. Pass `TOONLIVRE_BUNDLE_URL` with the direct bundle URL ' +
          `to bypass discovery. Original error: ${message}`
      );
    });
    const validation = await validateManifestAgainstChapter(manifest, canaryChapterUrl);

    expect(manifest.request.signatureHeader).toBe('x-toon-signature');
    expect(manifest.request.sessionCookie.name).toBe('toon_v');
    expect(validation.ok).toBe(true);
    expect(validation.pageCount).toBeGreaterThan(0);
    expect(Array.isArray(validation.decrypted.pages)).toBe(true);
  });
});
