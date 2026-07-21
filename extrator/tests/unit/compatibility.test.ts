import { basename, resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

import {
  buildCompatibilityFailureMessage,
  checkArchivedManifestCompatibility,
  checkBundleCompatibility,
} from '../../src/compatibility.js';

const manifestRoot = resolve(import.meta.dirname, '..', '..', 'manifest');
const manifestDir = resolve(manifestRoot, 'baselines');
const bundlesDir = resolve(import.meta.dirname, '..', '..', 'bundles');

describe('baseline manifest compatibility', () => {
  it('keeps every baseline bundle compatible with the current recognizers', async () => {
    const results = await checkArchivedManifestCompatibility({ manifestDir, bundlesDir });
    const failures = results.filter((result) => !result.ok);

    expect(results.length).toBeGreaterThan(0);
    expect(failures).toEqual([]);
  });

  it('compares an individual bundle against its baseline manifest', async () => {
    const bundleFile = resolve(
      bundlesDir,
      'bundle_v1784634648_index-CMe0Aw9p_js',
      'index-CMe0Aw9p.js'
    );
    const result = await checkBundleCompatibility({
      bundleFile,
      manifestDir,
      entryUrl: 'https://toonlivre.net/',
    });

    expect(result.expectedManifestPath.endsWith('index-CMe0Aw9p.json')).toBe(true);
    expect(result.ok).toBe(true);
    expect(result.actual.bundleFileName).toBe(basename(bundleFile));
    expect(result.actual.request.signatureStrategy).toEqual({
      kind: 'seed-jwt',
      metaName: 't-seed',
      endpointPath: '/api/seed',
      tokenField: 'token',
    });
    expect(buildCompatibilityFailureMessage(result)).toContain('Bundle compatibility check failed.');
  });
});
