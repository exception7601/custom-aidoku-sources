import { parse } from '@babel/parser';
import type { File } from '@babel/types';

export function parseBundle(bundleText: string): File {
  return parse(bundleText, {
    sourceType: 'module',
    allowReturnOutsideFunction: false,
    errorRecovery: false,
  });
}

export function snippetAround(source: string, needle: string, radius = 500): string | undefined {
  const index = source.indexOf(needle);
  if (index < 0) {
    return undefined;
  }

  return source.slice(Math.max(0, index - radius), Math.min(source.length, index + radius));
}
