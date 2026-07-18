import { describe, expect, it } from 'vitest';

import { analyzeHtmlDocument, isProtectedHtml } from '../../src/html.js';
import { readFixture } from '../helpers.js';

describe('HTML discovery', () => {
  it('collects script URLs from a document', async () => {
    const html = await readFixture('toonlivre-entry.html');
    const discovery = analyzeHtmlDocument(
      html,
      'https://toonlivre.net/contos-de-demonios-e-deuses/522.5'
    );

    expect(discovery.blockedByProtection).toBe(false);
    expect(discovery.scriptUrls).toEqual([
      'https://toonlivre.net/assets/index-DO83yVWS.js',
      'https://static.cloudflareinsights.com/beacon.min.js',
      'https://cdn.toonlivre.net/138219.js',
    ]);
  });

  it('flags Cloudflare-style challenge pages', () => {
    expect(
      isProtectedHtml(
        '<html><body><a href="https://toonlivre.net/cdn-cgi/content?id=abc">302</a></body></html>'
      )
    ).toBe(true);
  });
});
