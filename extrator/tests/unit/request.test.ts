import { describe, expect, it } from 'vitest';

import { parseBundle } from '../../src/ast.js';
import { recognizeRequestSignals } from '../../src/recognizers/request.js';
import { readFixture } from '../helpers.js';

const dynamicBundleSnippet = [
  "const ov = () => nl('toon_v') || '';",
  'const He = async (input, init = {}) => {',
  "  const i = typeof input === 'string' ? input : input.url;",
  '  const buildHeaders = async (requestInit) => {',
  '    const headers = new Headers(requestInit.headers || {});',
  "    const j = btoa(Math.PI.toString().substring(0, 5)) + '_' + '1388';",
  '    const D = Math.floor(Date.now() / 3e4);',
  "    const M = i.includes('/chapters') ? 'chapters' : 'other';",
  '    const $ = `${D}:${M}:${j}`;',
  '    const U = To.SHA256($).toString();',
  '    const V = btoa(`${D}:${U}`);',
  "    headers.append('x-toon-signature', V);",
  "    headers.append('x-toon-verify', ov());",
  '    return headers;',
  '  };',
  '',
  '  return buildHeaders(init);',
  '};',
  "const x = response.headers.get('x-toon-datakey');",
].join('\n');

describe('request recognizer', () => {
  it('extracts signature, verify, and datakey headers', async () => {
    const bundle = await readFixture('toonlivre-bundle-snippet.js');
    const recognition = recognizeRequestSignals(parseBundle(bundle));

    expect(recognition.signatureHeader).toBe('x-toon-signature');
    expect(recognition.signatureRules).toEqual([
      {
        when: {
          urlContains: '/chapters',
        },
        value: 't8v_authX9',
      },
      {
        default: true,
        value: 't8v_decoy9',
      },
    ]);
    expect(recognition.verifyHeader).toBe('x-toon-verify');
    expect(recognition.verifyFunctionName).toBe('ov');
    expect(recognition.dataKeyHeader).toBe('x-toon-datakey');
  });

  it('recognizes the dynamic chapter signature strategy', () => {
    const recognition = recognizeRequestSignals(parseBundle(dynamicBundleSnippet));

    expect(recognition.signatureHeader).toBe('x-toon-signature');
    expect(recognition.signatureRules).toEqual([]);
    expect(recognition.signatureStrategy).toEqual({
      kind: 'time-sha256-base64',
      timestampDivisor: 30000,
      salt: 'My4xNDE=_1388',
      routeSelector: {
        whenUrlContains: '/chapters',
        whenMatched: 'chapters',
        otherwise: 'other',
      },
    });
    expect(recognition.verifyHeader).toBe('x-toon-verify');
    expect(recognition.dataKeyHeader).toBe('x-toon-datakey');
  });

  it('recognizes the seed-backed signature strategy', async () => {
    const bundle = await readFixture('toonlivre-bundle-seed-snippet.js');
    const recognition = recognizeRequestSignals(parseBundle(bundle));

    expect(recognition.signatureHeader).toBe('x-toon-signature');
    expect(recognition.signatureRules).toEqual([]);
    expect(recognition.signatureStrategy).toEqual({
      kind: 'seed-jwt',
      metaName: 't-seed',
      endpointPath: '/api/seed',
      tokenField: 'token',
    });
    expect(recognition.verifyHeader).toBeUndefined();
    expect(recognition.dataKeyHeader).toBe('x-toon-datakey');
  });
});
