/** Why the files link ended, as the operator reads it. Every text is fixed: no path, address, token or device text. */
export type FilesEnd = 'closed' | 'hub-restarting' | 'refused-message' | 'failed' | 'went-quiet' | 'not-opened' | 'lost' | 'misread' | 'ended';

const TEXTS: Readonly<Record<FilesEnd, string>> = {
  closed: 'The file connection closed.',
  'hub-restarting': 'The hub is restarting.',
  'refused-message': 'The hub refused a message from this client.',
  failed: 'The connection to the device failed.',
  'went-quiet': 'The connection went quiet.',
  'not-opened': 'The file connection could not be opened.',
  lost: 'The hub stopped answering.',
  misread: 'The hub sent a message this client does not understand.',
  ended: 'The file connection ended unexpectedly.',
};

/** The hub's close codes for the files tunnel (K4 §9, WP-27 handoff). */
const BY_CODE: Readonly<Partial<Record<number, FilesEnd>>> = {
  1000: 'closed',
  1001: 'hub-restarting',
  1002: 'refused-message',
  1011: 'failed',
  4002: 'went-quiet',
};

/** The end a close with `code` means; before the upgrade succeeded every close is a refused opening (K4 §8.1). */
export function endOfClose(code: number, opened: boolean): FilesEnd {
  if (!opened) return 'not-opened';
  return BY_CODE[code] ?? 'ended';
}

/** The fixed operator text of `end`. */
export function endText(end: FilesEnd): string {
  return TEXTS[end];
}
