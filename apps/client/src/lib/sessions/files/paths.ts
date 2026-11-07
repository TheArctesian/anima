import type { FileEntry } from './link/codec';

/**
 * `directory` and `name` as one path, or `undefined` when `name` is not one plain name: empty, `.`, `..`, or holding
 * `/` or NUL. Names come from a device and are never trusted as paths of their own.
 */
export function joinPath(directory: string, name: string): string | undefined {
  if (name === '' || name === '.' || name === '..' || name.includes('/') || name.includes('\0')) return undefined;
  return directory.endsWith('/') ? `${directory}${name}` : `${directory}/${name}`;
}

/** The directory above the resolved `path`; the root and a bare name stay where they are or give `.`. */
export function parentPath(path: string): string {
  const trimmed = path.replace(/\/+$/, '');
  if (trimmed === '') return path.startsWith('/') ? '/' : '.';
  const cut = trimmed.lastIndexOf('/');
  if (cut < 0) return '.';
  return cut === 0 ? '/' : trimmed.slice(0, cut);
}

const FILE_NAME_MAX_BYTES = 255;
const utf8 = new TextEncoder();
/** Characters that are not shown or that reorder what is shown: controls (Cc) and format characters (Cf: bidi, zero-width, BOM). */
const UNSEEN = /[\p{Cc}\p{Cf}]/gu;

/** `text` cut at a code point so that it holds at most `bytes` bytes of UTF-8. */
function cutBytes(text: string, bytes: number): string {
  let kept = '';
  let used = 0;
  for (const point of text) {
    used += utf8.encode(point).byteLength;
    if (used > bytes) break;
    kept += point;
  }
  return kept;
}

/**
 * The file name to save a download under: the last segment of `name`, with no path separator, control or format
 * character, no leading dot and no trailing dot or space, at most 255 bytes of UTF-8 with its extension kept;
 * `download` when nothing is left.
 */
export function saveName(name: string): string {
  const last = name.split(/[/\\]/).at(-1) ?? '';
  const clean = last.replaceAll(UNSEEN, '').replace(/^[.\s]+/, '').replace(/[.\s]+$/, '');
  if (clean === '') return 'download';
  if (utf8.encode(clean).byteLength <= FILE_NAME_MAX_BYTES) return clean;
  const extension = /\.[^.]{1,16}$/.exec(clean)?.[0] ?? '';
  const stem = cutBytes(clean.slice(0, clean.length - extension.length), FILE_NAME_MAX_BYTES - utf8.encode(extension).byteLength);
  return `${stem.replace(/[.\s]+$/, '')}${extension}`;
}

/** Directories first, then the rest, each group by name in the operator's locale; the input is not changed. */
export function sortedEntries(entries: readonly FileEntry[]): FileEntry[] {
  const rank = (entry: FileEntry) => (entry.kind === 'dir' ? 0 : 1);
  return entries.toSorted((a, b) => rank(a) - rank(b) || a.name.localeCompare(b.name));
}

const UNITS = ['B', 'KiB', 'MiB', 'GiB', 'TiB'];

/** A byte count as text (`1.5 KiB`); `null`, which the device left out, is a dash. */
export function sizeText(size: number | null): string {
  if (size === null) return '—';
  let value = size;
  let unit = 0;
  while (value >= 1024 && unit < UNITS.length - 1) {
    value /= 1024;
    unit += 1;
  }
  const shown = unit === 0 || value >= 10 ? Math.round(value) : Math.round(value * 10) / 10;
  return `${String(shown)} ${UNITS[unit] ?? 'B'}`;
}

/** An RFC 3339 time as local date and time; `null` or a time that does not parse is a dash. */
export function timeText(modifiedAt: string | null, locale?: string): string {
  if (modifiedAt === null) return '—';
  const at = new Date(modifiedAt);
  return Number.isNaN(at.getTime()) ? '—' : at.toLocaleString(locale, { dateStyle: 'short', timeStyle: 'short' });
}

/** How far a transfer is, from 0 to 1, linear in the bytes moved (MOT-009); an unknown or empty size is 0. */
export function fraction(done: number, size: number | undefined): number {
  return size === undefined || size === 0 ? 0 : Math.min(1, done / size);
}
