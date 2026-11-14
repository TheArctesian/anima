import type { IconName } from '$lib/icons/icons';
import type { CodeEntryKind } from '../forge/types';

export const ENTRY_ICONS: Readonly<Record<CodeEntryKind, IconName>> = {
  DIRECTORY: 'carbon:folder',
  FILE: 'carbon:document',
  SYMLINK: 'carbon:link',
  SUBMODULE: 'carbon:folder-shared',
};

/** The way back from `path` to the root: the root, then each folder, then the path itself. */
export function crumbs(path: string): { name: string; path: string }[] {
  const parts = path === '' ? [] : path.split('/');
  return [{ name: 'Root', path: '' }, ...parts.map((name, index) => ({ name, path: parts.slice(0, index + 1).join('/') }))];
}

/** A file's size in bytes, KB or MB, as the file list shows it. */
export function sizeWords(bytes: number): string {
  if (bytes < 1024) return `${String(bytes)} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
