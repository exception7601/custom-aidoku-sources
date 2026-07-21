import { readFile, readdir } from 'node:fs/promises';
import { basename, join, resolve } from 'node:path';

import { DEFAULT_SITE_URL, DEFAULT_SOURCE_ID } from './constants.js';
import { extractManifest } from './extract.js';
import { type ExtractedManifest, parseManifest } from './manifest.js';
import { sha256 } from './http.js';

const DEFAULT_BASELINE_MANIFESTS_DIR = resolve('manifest/baselines');
const DEFAULT_BUNDLES_DIR = resolve('bundles');

interface BundleSnapshotRecord {
  directory: string;
  bundleFile: string;
  bundleFileName: string;
  bundleHash: string;
}

export interface BundleCompatibilityCheckOptions {
  bundleFile: string;
  manifestPath?: string;
  manifestDir?: string;
  sourceId?: string;
  siteUrl?: string;
  entryUrl?: string;
}

export interface BundleCompatibilityResult {
  bundleFile: string;
  bundleHash: string;
  expectedManifestPath: string;
  ok: boolean;
  actual: CompatibilityComparableManifest;
  expected: CompatibilityComparableManifest;
}

export interface BaselineCompatibilityCheckOptions {
  manifestDir?: string;
  bundlesDir?: string;
  sourceId?: string;
  siteUrl?: string;
}

export interface BaselineCompatibilityResult {
  manifestPath: string;
  bundleFile: string;
  bundleHash: string;
  ok: boolean;
  actual: CompatibilityComparableManifest;
  expected: CompatibilityComparableManifest;
}

export interface CompatibilityComparableManifest {
  schemaVersion: number;
  sourceId: string;
  siteUrl: string;
  bundleHash: string;
  bundleFileName?: string;
  request: {
    userAgent: string;
    acceptLanguage: string;
    signatureHeader: string;
    signatureRules: ExtractedManifest['request']['signatureRules'];
    signatureStrategy: ExtractedManifest['request']['signatureStrategy'] | null;
    verifyHeader: string | null;
    includeCredentials: boolean;
    sessionCookie: {
      name: string;
      generator: ExtractedManifest['request']['sessionCookie']['generator'];
      mirrorsInto: string[];
    };
  };
  decrypt: ExtractedManifest['decrypt'];
}

export async function checkBundleCompatibility(
  options: BundleCompatibilityCheckOptions
): Promise<BundleCompatibilityResult> {
  const bundleFile = resolve(options.bundleFile);
  const bundleText = await readFile(bundleFile, 'utf8');
  const bundleHash = sha256(bundleText);
  const expectedManifest = options.manifestPath
    ? await loadManifest(options.manifestPath)
    : await loadBaselineManifestForBundle(bundleFile, options.manifestDir);
  const actualManifest = await extractManifest({
    sourceId: options.sourceId ?? DEFAULT_SOURCE_ID,
    siteUrl: options.siteUrl ?? DEFAULT_SITE_URL,
    entryUrl: options.entryUrl,
    bundleFiles: [bundleFile],
  });

  const actual = normalizeManifestForCompatibility(actualManifest, basename(bundleFile));
  const expected = normalizeManifestForCompatibility(expectedManifest);

  return {
    bundleFile,
    bundleHash,
    expectedManifestPath: resolve(options.manifestPath ?? inferManifestPath(expectedManifest, options.manifestDir)),
    ok: manifestsMatch(actual, expected),
    actual,
    expected,
  };
}

export async function checkBaselineManifestCompatibility(
  options: BaselineCompatibilityCheckOptions = {}
): Promise<BaselineCompatibilityResult[]> {
  const manifestDir = resolve(options.manifestDir ?? DEFAULT_BASELINE_MANIFESTS_DIR);
  const bundlesDir = resolve(options.bundlesDir ?? DEFAULT_BUNDLES_DIR);
  const manifestPaths = (await readdir(manifestDir, { withFileTypes: true }))
    .filter((entry) => entry.isFile() && entry.name.endsWith('.json'))
    .map((entry) => join(manifestDir, entry.name))
    .sort();
  const bundleSnapshots = await loadBundleSnapshotRecords(bundlesDir);
  const results: BaselineCompatibilityResult[] = [];

  for (const manifestPath of manifestPaths) {
    const manifest = await loadManifest(manifestPath);
    const snapshot = findBundleSnapshotForManifest(bundleSnapshots, manifest);
    if (!snapshot) {
      throw new Error(`No saved bundle snapshot matched ${manifestPath}.`);
    }

    const actualManifest = await extractManifest({
      sourceId: options.sourceId ?? DEFAULT_SOURCE_ID,
      siteUrl: options.siteUrl ?? DEFAULT_SITE_URL,
      bundleFiles: [snapshot.bundleFile],
    });
    const actual = normalizeManifestForCompatibility(actualManifest, snapshot.bundleFileName);
    const expected = normalizeManifestForCompatibility(manifest);

    results.push({
      manifestPath,
      bundleFile: snapshot.bundleFile,
      bundleHash: snapshot.bundleHash,
      ok: manifestsMatch(actual, expected),
      actual,
      expected,
    });
  }

  return results;
}

function normalizeManifestForCompatibility(
  manifest: ExtractedManifest,
  fallbackBundleFileName?: string
): CompatibilityComparableManifest {
  return {
    schemaVersion: manifest.schemaVersion,
    sourceId: manifest.sourceId,
    siteUrl: manifest.siteUrl,
    bundleHash: manifest.bundle.hash,
    bundleFileName: fallbackBundleFileName ?? bundleFileNameFromManifest(manifest),
    request: {
      userAgent: manifest.request.userAgent,
      acceptLanguage: manifest.request.acceptLanguage,
      signatureHeader: manifest.request.signatureHeader,
      signatureRules: manifest.request.signatureRules,
      signatureStrategy: manifest.request.signatureStrategy ?? null,
      verifyHeader: manifest.request.verifyHeader ?? null,
      includeCredentials: manifest.request.includeCredentials,
      sessionCookie: {
        name: manifest.request.sessionCookie.name,
        generator: manifest.request.sessionCookie.generator,
        mirrorsInto: manifest.request.sessionCookie.mirrorsInto,
      },
    },
    decrypt: manifest.decrypt,
  };
}

function manifestsMatch(
  actual: CompatibilityComparableManifest,
  expected: CompatibilityComparableManifest
): boolean {
  return JSON.stringify(actual) === JSON.stringify(expected);
}

export function buildCompatibilityFailureMessage(
  result: {
    bundleFile?: string;
    manifestPath?: string;
    actual: CompatibilityComparableManifest;
    expected: CompatibilityComparableManifest;
  }
): string {
  return [
    'Bundle compatibility check failed.',
    result.bundleFile ? `Bundle: ${result.bundleFile}` : undefined,
    result.manifestPath ? `Expected manifest: ${result.manifestPath}` : undefined,
    'Expected:',
    JSON.stringify(result.expected, null, 2),
    'Actual:',
    JSON.stringify(result.actual, null, 2),
  ]
    .filter((line): line is string => line !== undefined)
    .join('\n');
}

async function loadManifest(manifestPath: string): Promise<ExtractedManifest> {
  const contents = await readFile(resolve(manifestPath), 'utf8');
  return parseManifest(JSON.parse(contents));
}

async function loadBaselineManifestForBundle(
  bundleFile: string,
  manifestDir: string | undefined
): Promise<ExtractedManifest> {
  const manifestPath = inferManifestPathFromBundleFile(bundleFile, manifestDir);
  return loadManifest(manifestPath);
}

function inferManifestPath(manifest: ExtractedManifest, manifestDir: string | undefined): string {
  return join(resolve(manifestDir ?? DEFAULT_BASELINE_MANIFESTS_DIR), manifestFileNameFromManifest(manifest));
}

function inferManifestPathFromBundleFile(bundleFile: string, manifestDir: string | undefined): string {
  return join(
    resolve(manifestDir ?? DEFAULT_BASELINE_MANIFESTS_DIR),
    `${bundleStem(basename(bundleFile))}.json`
  );
}

function manifestFileNameFromManifest(manifest: ExtractedManifest): string {
  return `${bundleStem(bundleFileNameFromManifest(manifest) ?? manifest.bundle.hash)}.json`;
}

function bundleFileNameFromManifest(manifest: ExtractedManifest): string | undefined {
  if (!manifest.bundle.url) {
    return undefined;
  }

  return basename(new URL(manifest.bundle.url).pathname);
}

function bundleStem(value: string): string {
  return value.replace(/\.js$/u, '');
}

async function loadBundleSnapshotRecords(bundlesDir: string): Promise<BundleSnapshotRecord[]> {
  const entries = await readdir(bundlesDir, { withFileTypes: true });
  const records: BundleSnapshotRecord[] = [];

  for (const entry of entries) {
    if (!entry.isDirectory()) {
      continue;
    }

    const directory = join(bundlesDir, entry.name);
    const metadataPath = join(directory, 'metadata.json');

    try {
      const metadata = JSON.parse(await readFile(metadataPath, 'utf8')) as {
        bundleFileName: string;
        bundleHash: string;
      };

      records.push({
        directory,
        bundleFile: join(directory, metadata.bundleFileName),
        bundleFileName: metadata.bundleFileName,
        bundleHash: metadata.bundleHash,
      });
    } catch {
      continue;
    }
  }

  return records;
}

function findBundleSnapshotForManifest(
  snapshots: BundleSnapshotRecord[],
  manifest: ExtractedManifest
): BundleSnapshotRecord | undefined {
  const byHash = snapshots.find((snapshot) => snapshot.bundleHash === manifest.bundle.hash);
  if (byHash) {
    return byHash;
  }

  const expectedBundleFileName = bundleFileNameFromManifest(manifest);
  return expectedBundleFileName
    ? snapshots.find((snapshot) => snapshot.bundleFileName === expectedBundleFileName)
    : undefined;
}
