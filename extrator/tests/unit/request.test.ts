import { describe, expect, it } from 'vitest';

import { parseBundle } from '../../src/ast.js';
import { recognizeRequestSignals } from '../../src/recognizers/request.js';
import { readFixture } from '../helpers.js';

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
});
