// Typed documents from the K1 schema (API-008). Runs under bun at build time, never in the client.
import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { generate } from '@graphql-codegen/cli';
import type { CodegenConfig } from '@graphql-codegen/cli';
import { preset } from '@graphql-codegen/client-preset';

/** Output folder, relative to the client root. */
const GENERATED_FOLDER = 'src/lib/hub/generated/';

/** Where generation reads and writes: the client root, the schema file and the document globs. */
interface TypesLayout {
  readonly clientRoot: string;
  readonly schema: string;
  readonly documents: readonly string[];
}

/** The client's own layout: the K1 schema and documents only from the domain clients (CQ-045). */
export function clientLayout(clientRoot: string): TypesLayout {
  return {
    clientRoot,
    schema: join(clientRoot, '..', '..', 'contracts', 'schema.graphql'),
    documents: ['src/lib/*/api.ts'],
  };
}

function codegenConfig(layout: TypesLayout): CodegenConfig {
  return {
    cwd: layout.clientRoot,
    schema: layout.schema,
    documents: [...layout.documents],
    ignoreNoDocuments: true,
    silent: true,
    generates: {
      [GENERATED_FOLDER]: {
        // The preset object, not its name, so that generation does not depend on the working folder.
        preset,
        presetConfig: { fragmentMasking: false },
        config: {
          strictScalars: true,
          scalars: { DateTime: 'string', IpAddr: 'string', Cidr: 'string', JSON: 'unknown', ID: 'string' },
          useTypeImports: true,
          enumsAsTypes: true,
          skipTypename: true,
        },
      },
    },
  };
}

/** The generated files, keyed by path relative to the client root; rejects when a document does not match the schema. */
async function generateTypes(layout: TypesLayout): Promise<Map<string, string>> {
  // `generate` is typed `any` because of its watch mode; without watch it resolves with the file list.
  const outputs: readonly { filename: string; content: string }[] = await generate(codegenConfig(layout), false);
  return new Map(outputs.map(({ filename, content }) => [filename, content]));
}

interface StaleType {
  readonly path: string;
  readonly reason: 'missing' | 'stale' | 'extra';
}

function staleTypes(built: ReadonlyMap<string, string>, clientRoot: string): StaleType[] {
  const folder = join(clientRoot, GENERATED_FOLDER);
  const onDisk = existsSync(folder) ? readdirSync(folder).map((name) => `${GENERATED_FOLDER}${name}`) : [];
  const extra = onDisk.filter((path) => !built.has(path)).map((path): StaleType => ({ path, reason: 'extra' }));
  const changed = [...built].flatMap(([path, text]): StaleType[] => {
    const file = join(clientRoot, path);
    if (!existsSync(file)) return [{ path, reason: 'missing' }];
    return readFileSync(file, 'utf8') === text ? [] : [{ path, reason: 'stale' }];
  });
  return [...changed, ...extra];
}

/** Generated files on disk that are missing, differ from a fresh build, or are no longer built (API-008). */
export async function checkTypes(layout: TypesLayout): Promise<StaleType[]> {
  return staleTypes(await generateTypes(layout), layout.clientRoot);
}

/** Replace the generated folder with a fresh build; returns the written paths. Nothing is removed when generation fails. */
export async function writeTypes(layout: TypesLayout): Promise<string[]> {
  const built = await generateTypes(layout);
  rmSync(join(layout.clientRoot, GENERATED_FOLDER), { recursive: true, force: true });
  for (const [path, text] of built) {
    const file = join(layout.clientRoot, path);
    mkdirSync(dirname(file), { recursive: true });
    writeFileSync(file, text);
  }
  return [...built.keys()];
}
