import { describe, expect, it } from 'vitest';

import type { ExtractedManifest } from '../../src/manifest.js';
import {
  buildCanaryPayloadFailureMessage,
  buildCanaryRequestFailureMessage,
} from '../../src/validate.js';

function makeManifest(): ExtractedManifest {
  return {
    schemaVersion: 1,
    sourceId: 'pt_BR.toonlivre',
    siteUrl: 'https://toonlivre.net',
    entryUrl: 'https://toonlivre.net/',
    extractedAt: '2026-07-19T00:00:00.000Z',
    bundle: {
      url: 'https://toonlivre.net/assets/index-DO83yVWS.js',
      hash: 'abc123',
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

describe('validate error formatting', () => {
  it('formats 403 errors with request hints', () => {
    const headers = new Headers({
      'content-type': 'application/json; charset=utf-8',
      'cf-ray': 'abc-123',
      'ratelimit-remaining': '0',
      'ratelimit-reset': '841',
    });
    const message = buildCanaryRequestFailureMessage({
      manifest: makeManifest(),
      chapterApiUrl: 'https://toonlivre.net/api/mangas/obra/chapters/cap',
      status: 403,
      body: JSON.stringify({ error: 'Acesso negado. Sessão inválida ou expirada.' }),
      responseHeaders: headers,
      signatureValue: 't8v_authX9',
    });

    expect(message).toContain('ToonLivre canary request failed.');
    expect(message).toContain('Status: 403 Forbidden');
    expect(message).toContain('Response error: Acesso negado. Sessão inválida ou expirada.');
    expect(message).toContain('Request signature: x-toon-signature=t8v_authX9');
    expect(message).toContain('Token mirror: x-toon-verify + cookie toon_v');
    expect(message).toContain('ratelimit-remaining=0');
    expect(message).toContain('Confira `x-toon-signature`, `x-toon-verify` e o cookie `toon_v`.');
  });

  it('formats payload failures with decrypt hints', () => {
    const message = buildCanaryPayloadFailureMessage({
      manifest: makeManifest(),
      chapterApiUrl: 'https://toonlivre.net/api/mangas/obra/chapters/cap',
      dataKey: '9ad941fe0ba341fa',
      body: '{"9ad941fe0ba341fa":"U2FsdGVkX1..."}',
      causeMessage: 'Failed to decrypt Rabbit payload.',
      stage: 'decrypt',
    });

    expect(message).toContain('ToonLivre canary response could not be decrypted.');
    expect(message).toContain('Data key: x-toon-datakey=9ad941fe0ba341fa');
    expect(message).toContain('Algorithm: cryptojs-rabbit');
    expect(message).toContain('Cause: Failed to decrypt Rabbit payload.');
    expect(message).toContain('passphrase pode ter ficado desatualizada');
  });
});
