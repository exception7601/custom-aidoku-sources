import { createHash } from 'node:crypto';

import { HTTP_ACCEPT_LANGUAGE, HTTP_USER_AGENT } from './constants.js';

export interface FetchTextOptions {
  headers?: HeadersInit;
  timeoutMs?: number;
}

export interface FetchTextResult {
  url: string;
  status: number;
  body: string;
}

export async function fetchText(
  url: string,
  options: FetchTextOptions = {}
): Promise<FetchTextResult> {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), options.timeoutMs ?? 15_000);

  try {
    const response = await fetch(url, {
      headers: {
        'user-agent': HTTP_USER_AGENT,
        'accept-language': HTTP_ACCEPT_LANGUAGE,
        accept: '*/*',
        ...options.headers,
      },
      redirect: 'follow',
      signal: controller.signal,
    });

    return {
      url: response.url,
      status: response.status,
      body: await response.text(),
    };
  } finally {
    clearTimeout(timeout);
  }
}

export function sha256(value: string): string {
  return createHash('sha256').update(value).digest('hex');
}

export function md5(value: string): string {
  return createHash('md5').update(value).digest('hex');
}
