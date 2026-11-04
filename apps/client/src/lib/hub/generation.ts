// Typed documents from the K1 schema (API-008). Runs under bun at build time, never in the client.
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { generate } from '@graphql-codegen/cli';
import type { CodegenConfig } from '@graphql-codegen/cli';

/** Output folder, relative to `apps/client`. */
export const GENERATED_FOLDER = 'src/lib/hub/generated/';

/** Documents live only in the domain clients (CQ-045). */
export const DOCUMENTS = ['src/lib/*/api.ts'];

/** Code Generator settings for `documents`, with paths relative to `clientRoot`. */
export function codegenConfig(clientRoot: string, documents: readonly string[]): CodegenConfig {
  return {
    cwd: clientRoot,
    schema: join(clientRoot, '..', '..', 'contracts', 'schema.graphql'),
    documents: [...documents],
    ignoreNoDocuments: true,
    silent: true,
    generates: {
      [GENERATED_FOLDER]: {
        preset: 'client',
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

/** The generated files for `documents`, keyed by path relative to `clientRoot`; rejects when a document does not match the schema. */
export async function generateTypes(clientRoot: string, documents: readonly string[] = DOCUMENTS): Promise<Map<string, string>> {
  // `generate` is typed `any` because of its watch mode; without watch it resolves with the file list.
  const outputs: readonly { filename: string; content: string }[] = await generate(codegenConfig(clientRoot, documents), false);
  return new Map(outputs.map(({ filename, content }) => [filename, content]));
}

export interface StaleType {
  readonly path: string;
  readonly reason: 'missing' | 'stale' | 'extra';
}

/** Generated files whose text on disk differs from `built` (API-008). */
export function staleTypes(built: ReadonlyMap<string, string>, clientRoot: string): StaleType[] {
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
