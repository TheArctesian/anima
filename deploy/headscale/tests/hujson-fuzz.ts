/**
 * Seeded fuzz run for the HuJSON reader. `hujson.test.ts` runs this file in a child
 * process with a timeout, because a synchronous loop that never ends cannot be stopped
 * from inside the test runner. Exit code 0 means every round behaved; 1 names a failure.
 */

import { HujsonError, parseHujson } from '../src/hujson.ts';

const TRICKY = ['', 'a', '// x', '/* y */', '*/', '"', '\\', ',]', '},', 'tag:hub:443'];
const SOUP = '{}[],:"/*\\\n 1a';

const failure = roundTrips(seeded(7), 500) ?? soupIsRefusedCleanly(seeded(11), 2000);
if (failure !== undefined) {
  console.error(failure);
  process.exitCode = 1;
}

function roundTrips(rand: () => number, rounds: number): string | undefined {
  for (let round = 0; round < rounds; round += 1) {
    const value = randomValue(rand, 3);
    const text = render(value, rand);
    if (JSON.stringify(parseHujson(text)) !== JSON.stringify(value)) return `round trip changed ${JSON.stringify(text)}`;
  }
  return undefined;
}

function soupIsRefusedCleanly(rand: () => number, rounds: number): string | undefined {
  for (let round = 0; round < rounds; round += 1) {
    const text = Array.from({ length: Math.floor(rand() * 24) }, () => SOUP.charAt(Math.floor(rand() * SOUP.length))).join('');
    try {
      parseHujson(text);
    } catch (error) {
      if (!(error instanceof HujsonError)) return `${JSON.stringify(text)} raised ${String(error)}`;
    }
  }
  return undefined;
}

function seeded(seed: number): () => number {
  let state = seed;
  return () => {
    state = (state * 1_103_515_245 + 12_345) % 2_147_483_648;
    return state / 2_147_483_648;
  };
}

function randomValue(rand: () => number, depth: number): unknown {
  const pick = Math.floor(rand() * (depth > 0 ? 6 : 4));
  if (pick === 0) return TRICKY[Math.floor(rand() * TRICKY.length)];
  if (pick === 1) return Math.floor(rand() * 1000) - 500;
  if (pick === 2) return rand() < 0.5;
  if (pick === 3) return null;
  const size = Math.floor(rand() * 4);
  if (pick === 4) return Array.from({ length: size }, () => randomValue(rand, depth - 1));
  return Object.fromEntries(Array.from({ length: size }, (_, index) => [`${TRICKY[index] ?? ''}${String(index)}`, randomValue(rand, depth - 1)]));
}

function render(value: unknown, rand: () => number): string {
  const noise = (): string => ['', ' ', '\n', '// c, ]\n', '/* , } */', '\t'][Math.floor(rand() * 6)] ?? '';
  const close = (parts: string[]): string => parts.join(',') + (parts.length > 0 && rand() < 0.5 ? `,${noise()}` : '') + noise();
  if (Array.isArray(value)) return `[${close(value.map((entry) => noise() + render(entry, rand) + noise()))}]`;
  if (typeof value === 'object' && value !== null) {
    const parts = Object.entries(value).map(([key, entry]) => `${noise()}${JSON.stringify(key)}${noise()}:${noise()}${render(entry, rand)}`);
    return `{${close(parts)}}`;
  }
  return JSON.stringify(value);
}
