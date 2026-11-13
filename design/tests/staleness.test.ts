import { afterEach, describe, expect, test } from 'bun:test';
import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { buildOutputs, OUTPUT_PATHS } from '../src/outputs.ts';
import { staleOutputs } from '../src/staleness.ts';

describe('staleOutputs on a repository root (TOK-002)', () => {
  const realRoot = resolve(import.meta.dir, '..', '..');
  const roots: string[] = [];

  afterEach(() => {
    for (const root of roots.splice(0)) rmSync(root, { recursive: true, force: true });
  });

  async function freshRoot(): Promise<string> {
    const root = mkdtempSync(join(tmpdir(), 'stale-'));
    roots.push(root);
    cpSync(join(realRoot, 'design', 'tokens'), join(root, 'design', 'tokens'), { recursive: true });
    const outputs = await buildOutputs(root);
    for (const [path, text] of Object.entries(outputs)) {
      mkdirSync(dirname(join(root, path)), { recursive: true });
      writeFileSync(join(root, path), text);
    }
    return root;
  }

  test('TOK-002 freshly written outputs are not stale', async () => {
    const root = await freshRoot();
    expect(await staleOutputs(root)).toEqual([]);
  });

  test('TOK-002 an output changed by one character is named as stale', async () => {
    const root = await freshRoot();
    const path = OUTPUT_PATHS.css;
    const file = join(root, path);
    const contents = readFileSync(file, 'utf8');
    writeFileSync(file, `${contents[0] === '!' ? '?' : '!'}${contents.slice(1)}`);
    expect(await staleOutputs(root)).toEqual([{ path, reason: 'stale' }]);
  });

  test('TOK-002 a deleted output is named as missing', async () => {
    const root = await freshRoot();
    const path = OUTPUT_PATHS.typescript;
    rmSync(join(root, path));
    expect(await staleOutputs(root)).toEqual([{ path, reason: 'missing' }]);
  });
});
