import { describe, expect, it } from 'vitest';

import type { ExtractedManifest } from '../../src/manifest.js';
import {
  classifyBundleProbe,
  classifyBundleUrlMatch,
  selectProbeCandidateUrls,
} from '../../src/probe.js';
import { readFixture } from '../helpers.js';
import { analyzeHtmlDocument } from '../../src/html.js';

function makeManifest(): ExtractedManifest {
  return {
    schemaVersion: 1,
    sourceId: 'pt_BR.toonlivre',
    siteUrl: 'https://toonlivre.net',
    entryUrl: 'https://toonlivre.net/',
    extractedAt: '2026-07-19T00:00:00.000Z',
    bundle: {
      url: 'https://toonlivre.net/assets/index-DO83yVWS.js',
      hash: 'bundle-hash',
      discoveredFrom: 'html',
    },
    request: {
      userAgent: 'Mozilla/5.0',
      acceptLanguage: 'en-US,en;q=0.9,pt;q=0.8',
      signatureHeader: 'x-toon-signature',
      signatureRules: [
        {
          value: 't8v_authX9',
          when: {
            urlContains: '/chapters',
          },
        },
        {
          value: 't8v_decoy9',
          default: true,
        },
      ],
      verifyHeader: 'x-toon-verify',
      includeCredentials: true,
      sessionCookie: {
        name: 'toon_v',
        generator: {
          kind: 'random-base36-concat',
          segments: [
            {
              radix: 36,
              start: 2,
              end: 15,
            },
          ],
        },
        mirrorsInto: ['x-toon-verify'],
      },
    },
    decrypt: {
      dataKeyHeader: 'x-toon-datakey',
      payloadSelector: 'header-named-or-first-string',
      algorithm: 'cryptojs-rabbit',
      passphrase: {
        kind: 'utc-md5-derived',
        dateFormat: 'YYYY-MM-DD',
        prefix: 'Dealer-Critter-Catnip4',
        salt: 'toonlivre.tv::v8',
        suffix: 't17_4v19_b2',
        digestEncoding: 'hex',
        digestSlice: {
          start: 0,
          end: 8,
        },
      },
    },
  };
}

describe('manifest bundle probe', () => {
  it('filters discovered scripts down to same-origin JavaScript candidates', async () => {
    const html = await readFixture('toonlivre-entry.html');
    const discovery = analyzeHtmlDocument(html, 'https://toonlivre.net/');
    const candidates = selectProbeCandidateUrls(discovery.scriptUrls, 'https://toonlivre.net');

    expect(candidates).toEqual(['https://toonlivre.net/assets/index-DO83yVWS.js']);
  });

  it('uses a direct bundle URL match as the fastest unchanged signal', () => {
    const manifest = makeManifest();
    const matchedBundleUrl = classifyBundleUrlMatch(manifest, [
      'https://toonlivre.net/assets/index-DO83yVWS.js',
    ]);

    expect(matchedBundleUrl).toBe('https://toonlivre.net/assets/index-DO83yVWS.js');
  });

  it('falls back to hash comparison when the URL changes', () => {
    const manifest = makeManifest();
    const probe = classifyBundleProbe({
      entryUrl: 'https://toonlivre.net/',
      manifest,
      candidateUrls: ['https://toonlivre.net/assets/index-new.js'],
      checkedBundles: [
        {
          url: 'https://toonlivre.net/assets/index-new.js',
          hash: 'bundle-hash',
        },
      ],
    });

    expect(probe.changed).toBe(false);
    expect(probe.reason).toBe('bundle-hash-match');
    expect(probe.matchedBundleUrl).toBe('https://toonlivre.net/assets/index-new.js');
  });

  it('marks the bundle as changed when neither URL nor hash matches', () => {
    const manifest = makeManifest();
    const probe = classifyBundleProbe({
      entryUrl: 'https://toonlivre.net/',
      manifest,
      candidateUrls: ['https://toonlivre.net/assets/index-new.js'],
      checkedBundles: [
        {
          url: 'https://toonlivre.net/assets/index-new.js',
          hash: 'different-hash',
        },
      ],
    });

    expect(probe.changed).toBe(true);
    expect(probe.reason).toBe('bundle-changed');
  });
});
