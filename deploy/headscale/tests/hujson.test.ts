import { describe, expect, test } from 'bun:test';
import { HujsonError, parseHujson } from '../src/hujson.ts';

const FUZZ_TIMEOUT_MS = 10_000;

describe('hujson', () => {
  test('vpn_004 comments inside strings are kept and trailing commas are dropped', () => {
    const text = '{"a": "// not a comment", /* c */ "b": ["/* kept */", 1,], // end\n}';
    expect(parseHujson(text)).toEqual({ a: '// not a comment', b: ['/* kept */', 1] });
  });

  test('vpn_004 a leading byte order mark is ignored', () => {
    expect(parseHujson('﻿{"a": 1,}')).toEqual({ a: 1 });
  });

  test('vpn_004 unterminated strings and comments and misplaced commas are refused', () => {
    for (const text of ['{"a": "x}', '{"a": 1 /* open', '[1,,]', '{,}', '[,]', ',1', '{"a":,1}']) {
      expect(() => parseHujson(text)).toThrow(HujsonError);
    }
  });

  test('vpn_004 seeded fuzz rounds read back unchanged or raise HujsonError, within a time bound', () => {
    const run = Bun.spawnSync([process.execPath, new URL('hujson-fuzz.ts', import.meta.url).pathname], { timeout: FUZZ_TIMEOUT_MS, stderr: 'pipe' });
    expect(run.signalCode ?? null).toBeNull();
    expect(run.stderr.toString()).toBe('');
    expect(run.exitCode).toBe(0);
  }, FUZZ_TIMEOUT_MS * 2);
});
