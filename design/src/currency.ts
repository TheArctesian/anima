import type { Outputs } from './outputs.ts';

export interface StaleFile {
  readonly path: string;
  readonly reason: 'missing' | 'stale';
}

/** Files whose text on disk differs from the freshly built text, in output order (TOK-002). */
export function staleFiles(built: Outputs, onDisk: Readonly<Record<string, string | undefined>>): StaleFile[] {
  return Object.entries(built).flatMap(([path, text]): StaleFile[] => {
    const current = onDisk[path];
    if (current === undefined) return [{ path, reason: 'missing' }];
    return current === text ? [] : [{ path, reason: 'stale' }];
  });
}
