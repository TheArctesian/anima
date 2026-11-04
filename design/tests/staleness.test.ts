import { afterEach, describe, expect, test } from 'bun:test';
import { cpSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { buildOutputs } from '../src/outputs.ts';
import { staleFiles, staleOutputs } from '../src/staleness.ts';

const built = { 'a.css': 'one', 'b.ts': 'two' };

describe('generated files current (TOK-002)', () => {
  test('TOK-002 passes when every file equals its build', () => {
    expect(staleFiles(built, { ...built })).toEqual([]);
  });

  test('TOK-002 names a file whose text changed by one character', () => {
    expect(staleFiles(built, { 'a.css': 'one', 'b.ts': 'twO' })).toEqual([{ path: 'b.ts', reason: 'stale' }]);
  });

  test('TOK-002 names a file that is missing', () => {
    expect(staleFiles(built, { 'a.css': 'one', 'b.ts': undefined })).toEqual([{ path: 'b.ts', reason: 'missing' }]);
  });
});

describe('staleOutputs on a repository root (TOK-002)', () => {
  const realRoot = resolve(import.meta.dir, '..', '..');
  const roots: string[] = [];

  afterEach(() => {
    for (const root of roots.splice(0)) rmSync(root, { recursive: true, force: true });
  });

  async function freshRoot(): Promise<{ root: string; paths: string[] }> {
    const root = mkdtempSync(join(tmpdir(), 'stale-'));
    roots.push(root);
    cpSync(join(realRoot, 'design', 'tokens'), join(root, 'design', 'tokens'), { recursive: true });
    const outputs = await buildOutputs(root);
    for (const [path, text] of Object.entries(outputs)) {
      mkdirSync(dirname(join(root, path)), { recursive: true });
      writeFileSync(join(root, path), text);
    }
    return { root, paths: Object.keys(outputs) };
  }

  test('TOK-002 freshly written outputs are not stale', async () => {
    const { root } = await freshRoot();
    expect(await staleOutputs(root)).toEqual([]);
  });

  test('TOK-002 an edited output is named as stale', async () => {
    const { root, paths } = await freshRoot();
    const path = paths[0] ?? '';
    writeFileSync(join(root, path), 'edited');
    expect(await staleOutputs(root)).toEqual([{ path, reason: 'stale' }]);
  });

  test('TOK-002 a deleted output is named as missing', async () => {
    const { root, paths } = await freshRoot();
    const path = paths[1] ?? '';
    rmSync(join(root, path));
    expect(await staleOutputs(root)).toEqual([{ path, reason: 'missing' }]);
  });
});
