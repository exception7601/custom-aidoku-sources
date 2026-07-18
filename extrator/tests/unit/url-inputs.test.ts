import { describe, expect, it } from 'vitest';

import { DEFAULT_SITE_URL } from '../../src/constants.js';
import { classifyBundleUrlInputs, isBaseSiteUrl } from '../../src/extract.js';

describe('bundle URL input classification', () => {
  it('treats the site root as a discovery URL', () => {
    expect(isBaseSiteUrl('https://toonlivre.net/', DEFAULT_SITE_URL)).toBe(true);
    expect(isBaseSiteUrl('https://toonlivre.net', DEFAULT_SITE_URL)).toBe(true);
    expect(isBaseSiteUrl('https://toonlivre.net/assets/index-DO83yVWS.js', DEFAULT_SITE_URL)).toBe(
      false
    );
  });

  it('splits direct bundle URLs from discovery roots', () => {
    const classified = classifyBundleUrlInputs(
      [
        'https://toonlivre.net/',
        'https://toonlivre.net/assets/index-DO83yVWS.js',
        'https://toonlivre.net/',
      ],
      DEFAULT_SITE_URL
    );

    expect(classified.discoveryEntryUrls).toEqual(['https://toonlivre.net/']);
    expect(classified.directBundleUrls).toEqual(['https://toonlivre.net/assets/index-DO83yVWS.js']);
  });
});
