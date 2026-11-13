import { describe, expect, it } from 'vitest';
import { isFailureLine, lastSequence, mergeLines, shownLines } from '../log';

const line = (sequence: number, text = `line ${String(sequence)}`) => ({ sequence, text });

describe('job log', () => {
  it('BLD-081: a line that arrives twice counts once and the lines stay in job order', () => {
    expect(mergeLines([line(1), line(2)], [line(3), line(2), line(0)]).map((item) => item.sequence)).toEqual([0, 1, 2, 3]);
  });

  it('BLD-081: the next page starts after the last line', () => {
    expect(lastSequence([line(4), line(7)])).toBe(7);
    expect(lastSequence([])).toBeNull();
  });

  it('BLD-049: a secret the hub missed is not shown', () => {
    const shown = shownLines([
      line(1, 'TAURI_SIGNING_PRIVATE_KEY=abc123'),
      line(2, 'PUT https://r2.example/x?X-Amz-Signature=deadbeef'),
      line(3, 'fine'),
    ]);
    expect(shown).toEqual(['TAURI_SIGNING_PRIVATE_KEY=[redacted]', 'PUT [redacted]', 'fine']);
  });

  it('BLD-049: a private key block is removed whole', () => {
    const shown = shownLines([
      line(1, '-----BEGIN PRIVATE KEY-----'),
      line(2, 'MIIEvQIBADANBg'),
      line(3, '-----END PRIVATE KEY-----'),
      line(4, 'after'),
    ]);
    expect(shown).toEqual(['[redacted]', '[redacted]', '[redacted]', 'after']);
  });

  it.each([
    ['@anima/client test: (fail) BLD-080 > releases [12ms]', true],
    ['error[E0308]: mismatched types', true],
    ["thread 'main' panicked at src/main.rs:3:5", true],
    ['@anima/deploy test: (pass) restore.sh failures leave the live database alone', false],
    ['cargo build --release', false],
  ])('BLD-081: %s is marked as a failure line: %s', (text, failure) => {
    expect(isFailureLine(text)).toBe(failure);
  });
});
