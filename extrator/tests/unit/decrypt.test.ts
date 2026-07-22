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

  it('extracts the SHA-256 passphrase recipe from the new bundle pattern', async () => {
    const bundle = await readFixture('toonlivre-bundle-seed-snippet.js');
    const recognition = recognizeDecryptSignals(parseBundle(bundle));

    expect(recognition.algorithm).toBe('cryptojs-rabbit');
    expect(recognition.passphraseFunctionName).toBe('sv');
    expect(recognition.passphrase).toEqual({
      kind: 'utc-sha256-derived',
      dateFormat: 'YYYY-MM-DD',
      prefix: 'Magnesium-Strike-Astonish3',
      salt: 'toonlivre.com::v8',
      suffix: 't8_4v2_b',
      digestEncoding: 'hex',
      digestSlice: {
        start: 0,
        end: 8,
      },
    });
  });

  it('extracts an inline string + slice passphrase recipe from the live bundle pattern', async () => {
    const bundle = await readFixture('toonlivre-bundle-seed-inline-passphrase-snippet.js');
    const recognition = recognizeDecryptSignals(parseBundle(bundle));

    expect(recognition.algorithm).toBe('cryptojs-rabbit');
    expect(recognition.passphraseFunctionName).toBe('sv');
    expect(recognition.passphrase).toEqual({
      kind: 'utc-sha256-derived',
      dateFormat: 'YYYY-MM-DD',
      prefix: 'Celestial-Raven-Invoke9',
      salt: 'toonlivre.net::v9p6_2x8_j',
      suffix: '',
      digestEncoding: 'hex',
      digestSlice: {
        start: 0,
        end: 8,
      },
    });
  });

  it('extracts an array-join + ISO date passphrase recipe from the latest live bundle pattern', async () => {
    const bundle = await readFixture('toonlivre-bundle-seed-array-join-snippet.js');
    const recognition = recognizeDecryptSignals(parseBundle(bundle));

    expect(recognition.algorithm).toBe('cryptojs-rabbit');
    expect(recognition.passphraseFunctionName).toBe('sv');
    expect(recognition.passphrase).toEqual({
      kind: 'utc-sha256-derived',
      dateFormat: 'YYYY-MM-DD',
      prefix: 'Phantom-Tide-Harvest8',
      salt: 'toonlivre.net::w3',
      suffix: 'r7_5m2_k',
      digestEncoding: 'hex',
      digestSlice: {
        start: 0,
        end: 8,
      },
    });
  });
});
