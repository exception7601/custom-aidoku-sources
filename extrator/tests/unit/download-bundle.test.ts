import { describe, expect, it } from 'vitest';

import {
  renderChangelogEntry,
  sanitizeBundleDirectoryName,
  sanitizeBundleFileName,
} from '../../src/download-bundle.js';

describe('bundle download helpers', () => {
  it('sanitizes bundle names for files and directories', () => {
    expect(sanitizeBundleFileName('index-CDYuwq2u.js')).toBe('index-CDYuwq2u.js');
    expect(sanitizeBundleDirectoryName('index-CDYuwq2u.js')).toBe('index-CDYuwq2u_js');
  });

  it('renders a changelog entry with signature guidance', () => {
    const entry = renderChangelogEntry(
      {
        downloadedAt: '2026-07-19T02:00:00.000Z',
        epochSeconds: 1784436000,
        bundleUrl: 'https://toonlivre.net/assets/index-CDYuwq2u.js',
        bundleFileName: 'index-CDYuwq2u.js',
        bundleHash: 'abc123',
        byteLength: 42,
        previousBundleDirectory: 'bundle_v1784435002_index-DO83yVWS_js',
        previousBundleHash: 'def456',
        previousBundleFileName: 'index-DO83yVWS.js',
      },
      {
        request: {
          signatureHeader: 'x-toon-signature',
          signatureRules: [],
          verifyHeader: 'x-toon-verify',
          verifyFunctionName: 'ov',
          dataKeyHeader: 'x-toon-datakey',
        },
        session: {
          cookieName: 'toon_v',
        },
        decrypt: {
          algorithm: 'cryptojs-rabbit',
        },
        snippets: {
          request: 'request snippet',
          session: 'session snippet',
          decrypt: 'decrypt snippet',
        },
      }
    );

    expect(entry).toContain('Hash changed: yes.');
    expect(entry).toContain('File name changed: yes.');
    expect(entry).toContain('no static rules recognized');
  });
});
