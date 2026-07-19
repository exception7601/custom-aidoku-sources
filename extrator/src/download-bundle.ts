import { mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
import { basename, join, resolve } from 'node:path';

import {
  DECRYPT_DEBUG_NEEDLE,
  DEFAULT_SITE_URL,
  REQUEST_DEBUG_NEEDLE,
  SESSION_DEBUG_NEEDLE,
} from './constants.js';
import { classifyBundleUrlInputs } from './extract.js';
import { analyzeHtmlDocument } from './html.js';
import { fetchText, sha256 } from './http.js';
import { parseBundle, snippetAround } from './ast.js';
import { recognizeDecryptSignals } from './recognizers/decrypt.js';
import { recognizeRequestSignals } from './recognizers/request.js';
import { recognizeSessionSignals } from './recognizers/session.js';

export interface DownloadBundleOptions {
  siteUrl?: string;
  entryUrl?: string;
  bundleUrls?: string[];
  outputDir?: string;
}

export interface BundleDownloadResult {
  directory: string;
  bundleFile: string;
  metadataFile: string;
  analysisFile: string;
  changelogFile: string;
  bundleUrl: string;
  bundleHash: string;
}

interface BundleMetadata {
  downloadedAt: string;
  epochSeconds: number;
  bundleUrl: string;
  bundleFileName: string;
  bundleHash: string;
  byteLength: number;
  previousBundleDirectory?: string;
  previousBundleUrl?: string;
  previousBundleHash?: string;
  previousBundleFileName?: string;
}

interface BundleAnalysis {
  request: ReturnType<typeof recognizeRequestSignals> | {
    error: string;
  };
  session: ReturnType<typeof recognizeSessionSignals> | {
    error: string;
  };
  decrypt: ReturnType<typeof recognizeDecryptSignals> | {
    error: string;
  };
  snippets: {
    request: string;
    session: string;
    decrypt: string;
  };
}

interface PreviousBundleRecord {
  directoryName: string;
  metadata: BundleMetadata;
}

const DEFAULT_BUNDLES_DIR = resolve('bundles');
const CHANGELOG_FILE_NAME = 'CHANGELOG.md';

export async function downloadBundle(
  options: DownloadBundleOptions = {}
): Promise<BundleDownloadResult> {
  const outputDir = resolve(options.outputDir ?? DEFAULT_BUNDLES_DIR);
  await mkdir(outputDir, { recursive: true });

  const bundleUrl = await resolveBundleUrl(options);
  const response = await fetchText(bundleUrl);
  const bundleHash = sha256(response.body);
  const bundleFileName = sanitizeBundleFileName(basename(new URL(response.url).pathname));
  const epochSeconds = Math.floor(Date.now() / 1_000);
  const directoryName = `bundle_v${epochSeconds}_${sanitizeBundleDirectoryName(bundleFileName)}`;
  const directory = join(outputDir, directoryName);
  await mkdir(directory, { recursive: true });

  const bundleFile = join(directory, bundleFileName);
  const metadataFile = join(directory, 'metadata.json');
  const analysisFile = join(directory, 'analysis.json');
  const changelogFile = join(outputDir, CHANGELOG_FILE_NAME);

  await writeFile(bundleFile, response.body, 'utf8');

  const previousBundle = await findPreviousBundleRecord(outputDir, directoryName);
  const metadata: BundleMetadata = {
    downloadedAt: new Date(epochSeconds * 1_000).toISOString(),
    epochSeconds,
    bundleUrl: response.url,
    bundleFileName,
    bundleHash,
    byteLength: Buffer.byteLength(response.body, 'utf8'),
    previousBundleDirectory: previousBundle?.directoryName,
    previousBundleUrl: previousBundle?.metadata.bundleUrl,
    previousBundleHash: previousBundle?.metadata.bundleHash,
    previousBundleFileName: previousBundle?.metadata.bundleFileName,
  };
  await writeFile(metadataFile, `${JSON.stringify(metadata, null, 2)}\n`, 'utf8');

  const analysis = analyzeBundle(response.body);
  await writeFile(analysisFile, `${JSON.stringify(analysis, null, 2)}\n`, 'utf8');

  await updateChangelog(changelogFile, metadata, analysis);

  return {
    directory,
    bundleFile,
    metadataFile,
    analysisFile,
    changelogFile,
    bundleUrl: response.url,
    bundleHash,
  };
}

export function sanitizeBundleFileName(bundleFileName: string): string {
  return bundleFileName.replace(/[^A-Za-z0-9._-]+/g, '_');
}

export function sanitizeBundleDirectoryName(bundleFileName: string): string {
  return bundleFileName.replace(/[^A-Za-z0-9_-]+/g, '_');
}

export function renderChangelogEntry(metadata: BundleMetadata, analysis: BundleAnalysis): string {
  const previousLabel = metadata.previousBundleDirectory
    ? `Previous bundle: \`${metadata.previousBundleDirectory}\`.`
    : 'Previous bundle: none.';
  const changedHash = metadata.previousBundleHash
    ? metadata.previousBundleHash !== metadata.bundleHash
      ? 'Hash changed: yes.'
      : 'Hash changed: no.'
    : 'Hash changed: n/a.';
  const changedFile = metadata.previousBundleFileName
    ? metadata.previousBundleFileName !== metadata.bundleFileName
      ? 'File name changed: yes.'
      : 'File name changed: no.'
    : 'File name changed: n/a.';
  const signatureMode =
    'signatureRules' in analysis.request
      ? analysis.request.signatureRules.length > 0
        ? 'Signature mode: static rules recognized.'
        : 'Signature mode: no static rules recognized; inspect `analysis.json` for dynamic logic.'
      : `Signature mode: request analysis failed (${analysis.request.error}).`;

  return [
    `## ${metadata.downloadedAt} — \`${metadata.bundleFileName}\``,
    '',
    `- Bundle URL: \`${metadata.bundleUrl}\``,
    `- Saved folder: \`bundle_v${metadata.epochSeconds}_${sanitizeBundleDirectoryName(metadata.bundleFileName)}\``,
    `- SHA-256: \`${metadata.bundleHash}\``,
    `- Bytes: \`${metadata.byteLength}\``,
    `- ${previousLabel}`,
    `- ${changedHash}`,
    `- ${changedFile}`,
    `- ${signatureMode}`,
    '- Site notes: fill this section after reviewing the downloaded bundle.',
    '',
  ].join('\n');
}

async function resolveBundleUrl(options: DownloadBundleOptions): Promise<string> {
  const siteUrl = options.siteUrl ?? DEFAULT_SITE_URL;
  const classified = classifyBundleUrlInputs(options.bundleUrls, siteUrl);

  const directBundleUrl = classified.directBundleUrls[0];
  if (directBundleUrl) {
    return directBundleUrl;
  }

  const discoveryUrl = options.entryUrl ?? classified.discoveryEntryUrls[0] ?? siteUrl;
  const entryResponse = await fetchText(discoveryUrl, {
    headers: {
      accept: 'text/html,application/xhtml+xml',
    },
  });
  const discovery = analyzeHtmlDocument(entryResponse.body, entryResponse.url);
  const bundleUrl = discovery.scriptUrls.find((scriptUrl) => /\/assets\/index-.*\.js$/u.test(scriptUrl));

  if (!bundleUrl) {
    throw new Error('Could not discover a live bundle URL from the entry HTML.');
  }

  return bundleUrl;
}

function analyzeBundle(bundleText: string): BundleAnalysis {
  try {
    const ast = parseBundle(bundleText);
    const request = recognizeRequestSignals(ast);
    const session = recognizeSessionSignals(ast, request.verifyFunctionName);
    const decrypt = recognizeDecryptSignals(ast);

    return {
      request,
      session,
      decrypt,
      snippets: {
        request: snippetAround(bundleText, REQUEST_DEBUG_NEEDLE) ?? '',
        session: snippetAround(bundleText, SESSION_DEBUG_NEEDLE) ?? '',
        decrypt: snippetAround(bundleText, DECRYPT_DEBUG_NEEDLE) ?? '',
      },
    };
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);

    return {
      request: {
        error: message,
      },
      session: {
        error: message,
      },
      decrypt: {
        error: message,
      },
      snippets: {
        request: snippetAround(bundleText, REQUEST_DEBUG_NEEDLE) ?? '',
        session: snippetAround(bundleText, SESSION_DEBUG_NEEDLE) ?? '',
        decrypt: snippetAround(bundleText, DECRYPT_DEBUG_NEEDLE) ?? '',
      },
    };
  }
}

async function findPreviousBundleRecord(
  outputDir: string,
  currentDirectoryName: string
): Promise<PreviousBundleRecord | undefined> {
  const entries = await readdir(outputDir, { withFileTypes: true });
  const bundleDirectories = entries
    .filter((entry) => entry.isDirectory() && entry.name.startsWith('bundle_v'))
    .map((entry) => entry.name)
    .filter((entryName) => entryName !== currentDirectoryName)
    .sort()
    .reverse();

  for (const directoryName of bundleDirectories) {
    const metadataPath = join(outputDir, directoryName, 'metadata.json');

    try {
      const metadataContents = await readFile(metadataPath, 'utf8');
      return {
        directoryName,
        metadata: JSON.parse(metadataContents) as BundleMetadata,
      };
    } catch {
      continue;
    }
  }

  return undefined;
}

async function updateChangelog(
  changelogFile: string,
  metadata: BundleMetadata,
  analysis: BundleAnalysis
): Promise<void> {
  const heading = '# Bundle changelog\n\n';
  const entry = renderChangelogEntry(metadata, analysis);
  let existing = '';

  try {
    existing = await readFile(changelogFile, 'utf8');
  } catch {
    existing = heading;
  }

  const contents = existing.startsWith('# Bundle changelog')
    ? `${heading}${entry}${stripHeading(existing)}`
    : `${heading}${entry}${existing}`;

  await writeFile(changelogFile, contents, 'utf8');
}

function stripHeading(contents: string): string {
  return contents.replace(/^# Bundle changelog\n\n/u, '');
}

