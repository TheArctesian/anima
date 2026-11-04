import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { afterAll, describe, expect, it } from 'vitest';
import { GENERATED_FOLDER, generateTypes, staleTypes } from './generation';

const clientRoot = resolve(import.meta.dirname, '..', '..', '..');
const scratch = mkdtempSync(join(tmpdir(), 'anima-codegen-'));

afterAll(() => {
  rmSync(scratch, { recursive: true, force: true });
});

function documentFile(name: string, query: string): string {
  const file = join(scratch, name);
  writeFileSync(file, `import { graphql } from '$lib/hub/generated';\nexport const Doc = graphql(\`${query}\`);\n`);
  return file;
}

describe('generated types', () => {
  it('API-008: the generated folder is current', async () => {
    expect(staleTypes(await generateTypes(clientRoot), clientRoot)).toEqual([]);
  });

  it('API-008: a document that matches the schema yields a typed document', async () => {
    const file = documentFile('known.ts', 'query Me { me { username } }');
    const built = await generateTypes(clientRoot, [file]);
    expect(built.get(`${GENERATED_FOLDER}graphql.ts`)).toContain('MeQuery');
  });

  it('API-008: a document with an unknown field fails generation', async () => {
    const file = documentFile('unknown.ts', 'query Me { me { nickname } }');
    await expect(generateTypes(clientRoot, [file])).rejects.toThrow(/Cannot query field "nickname"/);
  });

  it('API-008: a folder with a file the generator no longer writes is stale', async () => {
    const built = await generateTypes(clientRoot);
    const fewer = new Map([...built].slice(1));
    const [first] = [...built.keys()];
    expect(staleTypes(fewer, clientRoot)).toEqual([{ path: first, reason: 'extra' }]);
  });
});
