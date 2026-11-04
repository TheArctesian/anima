import { afterEach, describe, expect, test } from 'bun:test';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { loadTokens } from '../src/load.ts';
import { TokenSourceError } from '../src/model.ts';

const dirs: string[] = [];

afterEach(() => {
  for (const dir of dirs.splice(0)) rmSync(dir, { recursive: true, force: true });
});

async function loadSource(source: unknown): Promise<unknown> {
  const dir = mkdtempSync(join(tmpdir(), 'tokens-'));
  dirs.push(dir);
  writeFileSync(join(dir, 'motion.tokens.json'), typeof source === 'string' ? source : JSON.stringify(source));
  return loadTokens(dir).catch((error: unknown) => error);
}

describe('token source validation', () => {
  test('TOK-001 a spring without a numeric responseS is a TokenSourceError naming the token', async () => {
    const result = await loadSource({
      motion: { spring: { $type: 'spring', broken: { $value: { dampingRatio: 1, responseS: 'fast' } } } },
    });
    expect(result).toBeInstanceOf(TokenSourceError);
    expect((result as TokenSourceError).tokenPath).toBe('motion.spring.broken');
  });

  test('TOK-001 a cubic-bezier with three numbers is a TokenSourceError', async () => {
    const result = await loadSource({
      motion: { ease: { $type: 'cubicBezier', broken: { $value: [0.2, 0, 1] } } },
    });
    expect(result).toBeInstanceOf(TokenSourceError);
  });

  test('TOK-001 a spring with a non-finite response (1e999 parses to Infinity) is a TokenSourceError', async () => {
    const result = await loadSource(
      '{"motion":{"spring":{"$type":"spring","huge":{"$value":{"dampingRatio":1,"responseS":1e999}}}}}',
    );
    expect(result).toBeInstanceOf(TokenSourceError);
    expect((result as TokenSourceError).tokenPath).toBe('motion.spring.huge');
  });

  test('TOK-001 a cubic-bezier with a non-finite number is a TokenSourceError', async () => {
    const result = await loadSource('{"motion":{"ease":{"$type":"cubicBezier","huge":{"$value":[0.2,0,1e999,1]}}}}');
    expect(result).toBeInstanceOf(TokenSourceError);
  });

  test('TOK-001 a valid spring loads', async () => {
    const result = await loadSource({
      motion: { spring: { $type: 'spring', fine: { $value: { dampingRatio: 1, responseS: 0.35 } } } },
    });
    expect(Array.isArray(result)).toBe(true);
  });
});
