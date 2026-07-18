import { load } from 'cheerio';

export interface HtmlDiscoveryResult {
  scriptUrls: string[];
  blockedByProtection: boolean;
  reason?: string;
}

export function analyzeHtmlDocument(html: string, baseUrl: string): HtmlDiscoveryResult {
  const blockedByProtection = isProtectedHtml(html);
  const $ = load(html);
  const scriptUrls = Array.from(
    new Set(
      $('script[src]')
        .toArray()
        .map((element) => $(element).attr('src'))
        .filter((src): src is string => typeof src === 'string' && src.trim().length > 0)
        .map((src) => new URL(src, baseUrl).toString())
    )
  );

  return {
    scriptUrls,
    blockedByProtection,
    reason: blockedByProtection
      ? 'HTML response looks like a Cloudflare or redirect gate.'
      : undefined,
  };
}

export function isProtectedHtml(html: string): boolean {
  const normalized = html.toLowerCase();

  return (
    normalized.includes('cdn-cgi/content?id=') ||
    normalized.includes('https://www.mangalivre.net') ||
    normalized.includes('<title>302 found</title>')
  );
}
