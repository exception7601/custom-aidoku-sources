import { describe, expect, it } from 'vitest';

import { parseBundle } from '../../src/ast.js';
import { recognizeDecryptSignals } from '../../src/recognizers/decrypt.js';
import { readFixture } from '../helpers.js';

describe('decrypt recognizer', () => {
  it('extracts the Rabbit decryption strategy and passphrase recipe', async () => {
    const bundle = await readFixture('toonlivre-bundle-snippet.js');
    const recognition = recognizeDecryptSignals(parseBundle(bundle));

    expect(recognition.algorithm).toBe('cryptojs-rabbit');
    expect(recognition.passphraseFunctionName).toBe('nv');
    expect(recognition.passphrase).toEqual({
      kind: 'utc-md5-derived',
      dateFormat: 'YYYY-MM-DD',
      prefix: 'Dealer-Critter-Catnip4',
      salt: 'toonlivre.tv::v8',
      suffix: 't17_4v19_b2',
      digestEncoding: 'hex',
      digestSlice: {
        start: 0,
        end: 8,
      },
    });
  });
});
