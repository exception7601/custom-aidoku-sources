import { readFile } from 'node:fs/promises';

import { parseBundle, snippetAround } from './ast.js';
import {
  DECRYPT_DEBUG_NEEDLE,
  HTTP_ACCEPT_LANGUAGE,
  HTTP_USER_AGENT,
  REQUEST_DEBUG_NEEDLE,
  SESSION_DEBUG_NEEDLE,
} from './constants.js';
import { analyzeHtmlDocument } from './html.js';
import { fetchText, sha256 } from './http.js';
import { type ExtractedManifest, parseManifest } from './manifest.js';
import { recognizeDecryptSignals } from './recognizers/decrypt.js';
import { recognizeRequestSignals } from './recognizers/request.js';
import { recognizeSessionSignals } from './recognizers/session.js';

export interface ExtractManifestOptions {
  sourceId: string;
  siteUrl: string;
  entryUrl?: string;
  bundleUrls?: string[];
  bundleFiles?: string[];
}

export interface ClassifiedBundleUrlInputs {
  directBundleUrls: string[];
  discoveryEntryUrls: string[];
}

interface BundleCandidate {
  sourceKind: 'cli' | 'html' | 'file';
  location: string;
  text: string;
  hash: string;
}

interface CandidateRecognition {
  candidate: BundleCandidate;
  score: number;
  manifest?: ExtractedManifest;
  request: ReturnType<typeof recognizeRequestSignals>;
  session: ReturnType<typeof recognizeSessionSignals>;
  decrypt: ReturnType<typeof recognizeDecryptSignals>;
}

export function classifyBundleUrlInputs(
  bundleUrls: string[] | undefined,
  siteUrl: string
): ClassifiedBundleUrlInputs {
  const directBundleUrls: string[] = [];
  const discoveryEntryUrls: string[] = [];

  for (const bundleUrl of bundleUrls ?? []) {
    if (isBaseSiteUrl(bundleUrl, siteUrl)) {
      discoveryEntryUrls.push(new URL(bundleUrl).toString());
      continue;
    }

    directBundleUrls.push(bundleUrl);
  }

  return {
    directBundleUrls: Array.from(new Set(directBundleUrls)),
    discoveryEntryUrls: Array.from(new Set(discoveryEntryUrls)),
  };
}

export function isBaseSiteUrl(inputUrl: string, siteUrl: string): boolean {
  const input = new URL(inputUrl);
  const site = new URL(siteUrl);

  input.hash = '';
  input.search = '';
  site.hash = '';
  site.search = '';

  return (
    input.origin === site.origin && normalizePath(input.pathname) === normalizePath(site.pathname)
  );
}

export async function extractManifest(options: ExtractManifestOptions): Promise<ExtractedManifest> {
  const { candidates, discoveredScriptUrls, resolvedEntryUrl } = await loadBundleCandidates(options);
  if (candidates.length === 0) {
    throw new Error('No bundle candidates were loaded.');
  }

  const recognitionOptions = {
    ...options,
    entryUrl: resolvedEntryUrl,
  };
  const recognitions = candidates.map((candidate) =>
    buildRecognition(candidate, recognitionOptions, discoveredScriptUrls)
  );
  const bestRecognition = recognitions.sort((left, right) => right.score - left.score)[0];

  if (!bestRecognition || !bestRecognition.manifest) {
    throw new Error(buildIncompleteManifestError(bestRecognition));
  }

  return bestRecognition.manifest;
}

async function loadBundleCandidates(options: ExtractManifestOptions): Promise<{
  candidates: BundleCandidate[];
  discoveredScriptUrls: string[];
  resolvedEntryUrl?: string;
}> {
  const candidates: BundleCandidate[] = [];
  const discoveredScriptUrls: string[] = [];

  for (const bundleFile of options.bundleFiles ?? []) {
    const text = await readFile(bundleFile, 'utf8');
    candidates.push({
      sourceKind: 'file',
      location: bundleFile,
      text,
      hash: sha256(text),
    });
  }

  const classifiedBundleUrls = classifyBundleUrlInputs(options.bundleUrls, options.siteUrl);

  for (const bundleUrl of classifiedBundleUrls.directBundleUrls) {
    const response = await fetchText(bundleUrl);
    candidates.push({
      sourceKind: 'cli',
      location: response.url,
      text: response.body,
      hash: sha256(response.body),
    });
  }

  const discoveryEntryUrls = Array.from(
    new Set([...(options.entryUrl ? [options.entryUrl] : []), ...classifiedBundleUrls.discoveryEntryUrls])
  );

  if (candidates.length > 0 && discoveryEntryUrls.length === 0) {
    return {
      candidates,
      discoveredScriptUrls,
      resolvedEntryUrl: options.entryUrl,
    };
  }

  if (candidates.length === 0 && discoveryEntryUrls.length === 0) {
    throw new Error('No input provided. Use `--entry-url`, `--bundle-url`, or `--bundle-file`.');
  }

  for (const entryUrl of discoveryEntryUrls) {
    const htmlResponse = await fetchText(entryUrl, {
      headers: {
        accept: 'text/html,application/xhtml+xml',
      },
    });
    const discovery = analyzeHtmlDocument(htmlResponse.body, htmlResponse.url);
    discoveredScriptUrls.push(...discovery.scriptUrls);

    if (discovery.blockedByProtection && discovery.scriptUrls.length === 0) {
      throw new Error(
        discovery.reason + ' Pass `--bundle-url` with a live app bundle URL to continue.'
      );
    }

    for (const scriptUrl of discovery.scriptUrls) {
      const response = await fetchText(scriptUrl);
      candidates.push({
        sourceKind: 'html',
        location: response.url,
        text: response.body,
        hash: sha256(response.body),
      });
    }
  }

  return {
    candidates,
    discoveredScriptUrls,
    resolvedEntryUrl: discoveryEntryUrls[0] ?? options.entryUrl,
  };
}

function buildRecognition(
  candidate: BundleCandidate,
  options: ExtractManifestOptions,
  discoveredScriptUrls: string[]
): CandidateRecognition {
  const ast = parseBundle(candidate.text);
  const request = recognizeRequestSignals(ast);
  const decrypt = recognizeDecryptSignals(ast);
  const session = recognizeSessionSignals(ast, request.verifyFunctionName);

  const complete =
    request.signatureHeader &&
    (request.signatureRules.length > 0 || request.signatureStrategy) &&
    request.dataKeyHeader &&
    session.cookieName &&
    session.generator &&
    decrypt.algorithm &&
    decrypt.passphrase;

  const score =
    (request.signatureHeader ? 2 : 0) +
    (request.signatureRules.length > 0 || request.signatureStrategy ? 2 : 0) +
    (request.verifyHeader ? 1 : 0) +
    (request.dataKeyHeader ? 1 : 0) +
    (session.cookieName ? 2 : 0) +
    (session.generator ? 2 : 0) +
    (decrypt.algorithm ? 2 : 0) +
    (decrypt.passphrase ? 3 : 0);

  if (!complete) {
    return {
      candidate,
      score,
      request,
      session,
      decrypt,
    };
  }

  const manifest = parseManifest({
    schemaVersion: 1,
    sourceId: options.sourceId,
    siteUrl: options.siteUrl,
    entryUrl: options.entryUrl,
    extractedAt: new Date().toISOString(),
    bundle: {
      url: candidate.sourceKind === 'file' ? undefined : candidate.location,
      hash: candidate.hash,
      discoveredFrom: candidate.sourceKind,
    },
    request: {
      userAgent: HTTP_USER_AGENT,
      acceptLanguage: HTTP_ACCEPT_LANGUAGE,
      signatureHeader: request.signatureHeader,
      signatureRules: request.signatureRules,
      signatureStrategy: request.signatureStrategy,
      verifyHeader: request.verifyHeader,
      includeCredentials: true,
      sessionCookie: {
        name: session.cookieName,
        generator: session.generator,
        mirrorsInto: request.verifyHeader ? [request.verifyHeader] : [],
      },
    },
    decrypt: {
      dataKeyHeader: request.dataKeyHeader,
      payloadSelector: 'header-named-or-first-string',
      algorithm: decrypt.algorithm,
      passphrase: decrypt.passphrase,
    },
    diagnostics: {
      scriptUrls: discoveredScriptUrls,
      snippets: {
        request: snippetAround(candidate.text, REQUEST_DEBUG_NEEDLE),
        session: snippetAround(candidate.text, SESSION_DEBUG_NEEDLE),
        decrypt: snippetAround(candidate.text, DECRYPT_DEBUG_NEEDLE),
      },
    },
  });

  return {
    candidate,
    score,
    manifest,
    request,
    session,
    decrypt,
  };
}

function buildIncompleteManifestError(recognition: CandidateRecognition | undefined): string {
  if (!recognition) {
    return 'Unable to extract a complete manifest because no bundle recognition candidates were scored.';
  }

  const missing: string[] = [];
  if (!recognition.request.signatureHeader) {
    missing.push('request.signatureHeader');
  }
  if (
    recognition.request.signatureRules.length === 0 &&
    recognition.request.signatureStrategy === undefined
  ) {
    missing.push('request.signatureRules|signatureStrategy');
  }
  if (!recognition.request.dataKeyHeader) {
    missing.push('decrypt.dataKeyHeader');
  }
  if (!recognition.session.cookieName) {
    missing.push('request.sessionCookie.name');
  }
  if (!recognition.session.generator) {
    missing.push('request.sessionCookie.generator');
  }
  if (!recognition.decrypt.algorithm) {
    missing.push('decrypt.algorithm');
  }
  if (!recognition.decrypt.passphrase) {
    missing.push('decrypt.passphrase');
  }

  const recognized = [
    `candidate=${recognition.candidate.location}`,
    `signatureHeader=${recognition.request.signatureHeader ?? 'missing'}`,
    `signatureRules=${recognition.request.signatureRules.length}`,
    `signatureStrategy=${recognition.request.signatureStrategy?.kind ?? 'missing'}`,
    `verifyHeader=${recognition.request.verifyHeader ?? 'missing'}`,
    `cookieName=${recognition.session.cookieName ?? 'missing'}`,
    `cookieGenerator=${recognition.session.generator?.kind ?? 'missing'}`,
    `dataKeyHeader=${recognition.request.dataKeyHeader ?? 'missing'}`,
    `decryptAlgorithm=${recognition.decrypt.algorithm ?? 'missing'}`,
    `passphrase=${recognition.decrypt.passphrase?.kind ?? 'missing'}`,
  ];

  return [
    'Unable to extract a complete manifest from the best-scoring bundle candidate.',
    `Missing: ${missing.join(', ') || 'unknown'}`,
    `Recognized: ${recognized.join(', ')}`,
    'If this is the live site, inspect the saved `analysis.json` and extend the relevant recognizer.',
  ].join('\n');
}

function normalizePath(pathname: string): string {
  return pathname.endsWith('/') ? pathname : `${pathname}/`;
}
