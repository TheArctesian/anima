/** What the terminal does itself with a key combination instead of sending it to the device. */
export type ShortcutAction = 'copy' | 'find' | 'paste';

/** The parts of a key event the shortcuts read. */
export interface ShortcutKey {
  readonly key: string;
  readonly ctrlKey: boolean;
  readonly metaKey: boolean;
  readonly shiftKey: boolean;
  readonly altKey: boolean;
}

/**
 * The operator's own shortcuts (RA-003), which stay off the device:
 * - copy: Ctrl+Shift+C, or Cmd+C (a Ctrl+C alone is the interrupt and goes to the device);
 * - paste: Ctrl+Shift+V or Cmd+V, left to the browser's `paste` event, which xterm.js turns into bracketed paste;
 * - find: Cmd+F, or Ctrl+Shift+F (a Ctrl+F alone is the shell's "forward one character").
 */
export function shortcutOf(event: ShortcutKey): ShortcutAction | undefined {
  if (event.altKey) return undefined;
  const key = event.key.toLowerCase();
  const chorded = event.ctrlKey && event.shiftKey && !event.metaKey;
  const command = event.metaKey && !event.ctrlKey && !event.shiftKey;
  if (key === 'c' && (chorded || command)) return 'copy';
  if (key === 'v' && (chorded || command)) return 'paste';
  if (key === 'f' && (chorded || command)) return 'find';
  return undefined;
}
