import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { afterAll, describe, expect, it } from 'vitest';
import { checkTypes, clientLayout, writeTypes } from './generation';

const layout = clientLayout(resolve(import.meta.dirname, '..', '..', '..'));
const scratch = mkdtempSync(join(tmpdir(), 'anima-codegen-'));
const GENERATED = 'src/lib/hub/generated';

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
    expect(await checkTypes(layout)).toEqual([]);
  });

  it('API-008: a document that matches the schema yields a typed document', async () => {
    const root = join(scratch, 'known');
    mkdirSync(root);
    const file = documentFile('known.ts', 'query Me { me { username } }');
    await writeTypes({ ...layout, clientRoot: root, documents: [file] });
    expect(readFileSync(join(root, GENERATED, 'graphql.ts'), 'utf8')).toContain('MeQuery');
  });

  it('API-008: a document with an unknown field fails generation', async () => {
    const file = documentFile('unknown.ts', 'query Me { me { nickname } }');
    await expect(checkTypes({ ...layout, documents: [file] })).rejects.toThrow(/Cannot query field "nickname"/);
  });
});

describe('writing and checking generated types', () => {
  const root = join(scratch, 'client');
  const elsewhere = { ...layout, clientRoot: root, documents: [] };
  const folder = join(root, GENERATED);

  it('API-008: writing replaces the folder, so a file no longer generated is removed', async () => {
    mkdirSync(folder, { recursive: true });
    writeFileSync(join(folder, 'old.ts'), 'export {};\n');
    const written = await writeTypes(elsewhere);
    expect(written.toSorted()).toEqual([`${GENERATED}/gql.ts`, `${GENERATED}/graphql.ts`, `${GENERATED}/index.ts`]);
    expect(await checkTypes(elsewhere)).toEqual([]);
  });

  it('API-008: the check names changed, missing and extra files', async () => {
    await writeTypes(elsewhere);
    writeFileSync(join(folder, 'gql.ts'), '// edited by hand\n');
    rmSync(join(folder, 'index.ts'));
    writeFileSync(join(folder, 'extra.ts'), 'export {};\n');
    expect(await checkTypes(elsewhere)).toEqual(
      expect.arrayContaining([
        { path: `${GENERATED}/gql.ts`, reason: 'stale' },
        { path: `${GENERATED}/index.ts`, reason: 'missing' },
        { path: `${GENERATED}/extra.ts`, reason: 'extra' },
      ]),
    );
    expect(await checkTypes(elsewhere)).toHaveLength(3);
  });
});
