import { analyzeHtmlDocument, type HtmlDiscoveryResult } from './html.js';
import { fetchText, sha256 } from './http.js';
import type { ExtractedManifest } from './manifest.js';

const FAILURE_BODY_LIMIT = 220;

export interface ProbedBundleCandidate {
  url: string;
  status: number;
  hash: string;
}

export interface BundleProbeResult {
  entryUrl: string;
  entryStatus: number;
  manifestBundleUrl?: string;
  manifestBundleHash: string;
  candidateUrls: string[];
  checkedBundles: ProbedBundleCandidate[];
  changed: boolean;
  reason: 'bundle-url-match' | 'bundle-hash-match' | 'bundle-changed';
  matchedBundleUrl?: string;
}

export interface ProbeManifestBundleOptions {
  manifest: ExtractedManifest;
  siteUrl: string;
  entryUrl?: string;
}

export async function probeManifestBundle(
  options: ProbeManifestBundleOptions
): Promise<BundleProbeResult> {
  const entryUrl = options.entryUrl ?? options.siteUrl;
  const htmlResponse = await fetchText(entryUrl, {
    headers: {
      accept: 'text/html,application/xhtml+xml',
    },
  });
  const discovery = analyzeHtmlDocument(htmlResponse.body, htmlResponse.url);
  const candidateUrls = selectProbeCandidateUrls(discovery.scriptUrls, options.siteUrl);

  if (candidateUrls.length === 0) {
    throw new Error(
      buildNoBundleCandidatesError({
        entryUrl: htmlResponse.url,
        entryStatus: htmlResponse.status,
        discovery,
        body: htmlResponse.body,
      })
    );
  }

  const urlMatch = classifyBundleUrlMatch(options.manifest, candidateUrls);
  if (urlMatch) {
    return {
      entryUrl: htmlResponse.url,
      entryStatus: htmlResponse.status,
      manifestBundleUrl: options.manifest.bundle.url,
      manifestBundleHash: options.manifest.bundle.hash,
      candidateUrls,
      checkedBundles: [],
      changed: false,
      reason: 'bundle-url-match',
      matchedBundleUrl: urlMatch,
    };
  }

  const checkedBundles = await Promise.all(
    candidateUrls.map(async (candidateUrl) => {
      const response = await fetchText(candidateUrl);

      return {
        url: response.url,
        status: response.status,
        hash: sha256(response.body),
      } satisfies ProbedBundleCandidate;
    })
  );

  return classifyBundleProbe({
    entryUrl: htmlResponse.url,
    entryStatus: htmlResponse.status,
    manifest: options.manifest,
    candidateUrls,
    checkedBundles,
  });
}

export function selectProbeCandidateUrls(scriptUrls: string[], siteUrl: string): string[] {
  const siteOrigin = new URL(siteUrl).origin;

  return Array.from(
    new Set(
      scriptUrls.filter((scriptUrl) => {
        const url = new URL(scriptUrl);

        return url.origin === siteOrigin && url.pathname.endsWith('.js');
      })
    )
  );
}

export function classifyBundleUrlMatch(
  manifest: ExtractedManifest,
  candidateUrls: string[]
): string | undefined {
  const manifestBundleUrl = manifest.bundle.url;
  if (!manifestBundleUrl) {
    return undefined;
  }

  return candidateUrls.find((candidateUrl) => candidateUrl === manifestBundleUrl);
}

export function classifyBundleProbe(args: {
  entryUrl: string;
  entryStatus: number;
  manifest: ExtractedManifest;
  candidateUrls: string[];
  checkedBundles: ProbedBundleCandidate[];
}): BundleProbeResult {
  const hashMatch = args.checkedBundles.find(
    (checkedBundle) => checkedBundle.hash === args.manifest.bundle.hash
  );

  return {
    entryUrl: args.entryUrl,
    entryStatus: args.entryStatus,
    manifestBundleUrl: args.manifest.bundle.url,
    manifestBundleHash: args.manifest.bundle.hash,
    candidateUrls: args.candidateUrls,
    checkedBundles: args.checkedBundles,
    changed: hashMatch === undefined,
    reason: hashMatch ? 'bundle-hash-match' : 'bundle-changed',
    matchedBundleUrl: hashMatch?.url,
  };
}

export function buildNoBundleCandidatesError(args: {
  entryUrl: string;
  entryStatus: number;
  discovery: HtmlDiscoveryResult;
  body: string;
}): string {
  const lines = [
    'No same-origin JavaScript bundle candidates were discovered from the entry HTML.',
    `Entry URL: ${args.entryUrl}`,
    `Entry status: ${args.entryStatus}`,
    `Blocked by protection: ${args.discovery.blockedByProtection ? 'yes' : 'no'}`,
  ];
  const discoveredScripts =
    args.discovery.scriptUrls.length > 0 ? args.discovery.scriptUrls : ['none'];
  const bodySnippet = summarizeBody(args.body);

  lines.push(`Discovered scripts: ${discoveredScripts.join(', ')}`);

  if (args.discovery.reason) {
    lines.push(`Hint: ${args.discovery.reason}`);
  }

  if (bodySnippet) {
    lines.push(`Body: ${bodySnippet}`);
  }

  return lines.join('\n');
}

function summarizeBody(body: string): string {
  return body.replace(/\s+/g, ' ').trim().slice(0, FAILURE_BODY_LIMIT);
}
