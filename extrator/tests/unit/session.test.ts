import { describe, expect, it } from 'vitest';

import { parseBundle } from '../../src/ast.js';
import { recognizeSessionSignals } from '../../src/recognizers/session.js';
import { readFixture } from '../helpers.js';

describe('session recognizer', () => {
  it('extracts the mirrored cookie and generator strategy', async () => {
    const bundle = await readFixture('toonlivre-bundle-snippet.js');
    const recognition = recognizeSessionSignals(parseBundle(bundle), 'ov');

    expect(recognition.cookieName).toBe('toon_v');
    expect(recognition.generator).toEqual({
      kind: 'random-base36-concat',
      segments: [
        {
          radix: 36,
          start: 2,
          end: 15,
        },
        {
          radix: 36,
          start: 2,
          end: 15,
        },
      ],
    });
  });
});
