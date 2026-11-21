import type { Extension } from './api';
import { willDo } from './coming';

/** What a row of the Extensions section offers: a live switch, or the reason there is none. */
export type SwitchKind = 'switch' | 'unbuilt' | 'fixed' | 'needed';

export interface ExtensionRow {
  readonly key: string;
  readonly label: string;
  readonly hint: string;
  readonly kind: SwitchKind;
  readonly on: boolean;
}

function names(keys: readonly string[], all: readonly Extension[]): string[] {
  return keys.map((key) => all.find((each) => each.key === key)?.name ?? key);
}

function list(words: readonly string[]): string {
  return words.length < 2 ? (words[0] ?? '') : `${words.slice(0, -1).join(', ')} and ${words.at(-1) ?? ''}`;
}

function kindOf(extension: Extension): SwitchKind {
  if (!extension.built) return 'unbuilt';
  if (extension.fixed) return 'fixed';
  return extension.neededBy.length > 0 ? 'needed' : 'switch';
}

function hintOf(extension: Extension, kind: SwitchKind, all: readonly Extension[]): string {
  switch (kind) {
    case 'unbuilt':
      return willDo(extension.key);
    case 'fixed':
      return 'Always on.';
    case 'needed': {
      const dependants = list(names(extension.neededBy, all));
      return `${dependants} needs it. Set ${dependants} off first.`;
    }
    case 'switch':
      return extension.needs.length > 0 ? `Needs ${list(names(extension.needs, all))}` : '';
  }
}

/** The rows of the section, in the hub's order (EXT-012: a needed extension names its dependant). */
export function extensionRows(all: readonly Extension[]): ExtensionRow[] {
  return all.map((extension) => {
    const kind = kindOf(extension);
    return { key: extension.key, label: extension.name, hint: hintOf(extension, kind, all), kind, on: extension.on };
  });
}

/** The count line over the rows: how many extensions are on, and how many are not built yet (EXT-001). */
export function countLine(rows: readonly ExtensionRow[]): string {
  const on = rows.filter((row) => row.kind !== 'unbuilt' && row.on).length;
  const coming = rows.filter((row) => row.kind === 'unbuilt').length;
  return coming === 0 ? `${String(on)} on` : `${String(on)} on, ${String(coming)} coming`;
}

/** The sentence shown before a switch also sets needs on (EXT-011), or `null` when it sets only itself. */
export function alsoSetsSentence(key: string, all: readonly Extension[]): string | null {
  const extension = all.find((each) => each.key === key);
  if (extension === undefined) return null;
  const missing = extension.needs.filter((need) => all.find((each) => each.key === need)?.on !== true);
  return missing.length === 0 ? null : `${extension.name} also sets ${list(names(missing, all))} on`;
}
