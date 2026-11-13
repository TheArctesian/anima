import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { buildOutputs, type Outputs } from './outputs.ts';

export interface StaleFile {
  readonly path: string;
  readonly reason: 'missing' | 'stale';
}

/** Files whose text on disk differs from the freshly built text, in output order (TOK-002). */
function staleFiles(built: Outputs, onDisk: Readonly<Record<string, string | undefined>>): StaleFile[] {
  return Object.entries(built).flatMap(([path, text]): StaleFile[] => {
    const current = onDisk[path];
    if (current === undefined) return [{ path, reason: 'missing' }];
    return current === text ? [] : [{ path, reason: 'stale' }];
  });
}

/** Build every output from the token source under `repositoryRoot` and compare with the files there (TOK-002). */
export async function staleOutputs(repositoryRoot: string): Promise<StaleFile[]> {
  const built = await buildOutputs(repositoryRoot);
  const onDisk = Object.fromEntries(
    Object.keys(built).map((path) => {
      const file = join(repositoryRoot, path);
      return [path, existsSync(file) ? readFileSync(file, 'utf8') : undefined];
    }),
  );
  return staleFiles(built, onDisk);
}
