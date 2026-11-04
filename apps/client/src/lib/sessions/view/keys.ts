/** X11 keysyms the Keys menu sends (X11 `keysymdef.h`, as guacd reads them). */
export const KEYSYM = {
  controlLeft: 0xffe3,
  altLeft: 0xffe9,
  shiftLeft: 0xffe1,
  delete: 0xffff,
  escape: 0xff1b,
  tab: 0xff09,
  c: 0x63,
  d: 0x64,
  l: 0x6c,
  z: 0x7a,
  shiftedV: 0x56,
} as const;

/** The Meta and Super keys (Command on macOS): shortcuts with them belong to the operator's system, not the device. */
const SYSTEM_MODIFIERS: ReadonlySet<number> = new Set([0xffe7, 0xffe8, 0xffeb, 0xffec]);

/** One key event: 1 is a press, 0 a release (the values `sendKeyEvent` takes). */
export type KeyStroke = readonly [pressed: 0 | 1, keysym: number];

/** A key combination of the Keys menu (D-10, RA-006). */
export interface KeyCombination {
  readonly id: string;
  /** The name the menu shows, which is also the item's accessible name. */
  readonly label: string;
  /** What the combination usually does in a terminal. */
  readonly hint: string;
  /** The keys, pressed in this order and released in the reverse order. */
  readonly keysyms: readonly number[];
}

/** The Keys menu combinations, in menu order (D-10). "Paste" is not a combination: it sends the clipboard. */
export const KEY_COMBINATIONS: readonly KeyCombination[] = [
  { id: 'ctrl-c', label: 'Ctrl+C', hint: 'Interrupt', keysyms: [KEYSYM.controlLeft, KEYSYM.c] },
  { id: 'ctrl-d', label: 'Ctrl+D', hint: 'End of input', keysyms: [KEYSYM.controlLeft, KEYSYM.d] },
  { id: 'ctrl-z', label: 'Ctrl+Z', hint: 'Suspend', keysyms: [KEYSYM.controlLeft, KEYSYM.z] },
  { id: 'ctrl-l', label: 'Ctrl+L', hint: 'Clear the screen', keysyms: [KEYSYM.controlLeft, KEYSYM.l] },
  { id: 'esc', label: 'Esc', hint: 'Escape', keysyms: [KEYSYM.escape] },
  { id: 'tab', label: 'Tab', hint: 'Complete', keysyms: [KEYSYM.tab] },
  {
    id: 'ctrl-alt-delete',
    label: 'Ctrl+Alt+Delete',
    hint: 'Secure attention',
    keysyms: [KEYSYM.controlLeft, KEYSYM.altLeft, KEYSYM.delete],
  },
];

/** The press and release sequence of `combination`: every key pressed in order, then released in reverse. */
export function keyStrokes(combination: KeyCombination): readonly KeyStroke[] {
  const presses = combination.keysyms.map((keysym): KeyStroke => [1, keysym]);
  const releases = [...combination.keysyms].reverse().map((keysym): KeyStroke => [0, keysym]);
  return [...presses, ...releases];
}

/**
 * guacd's terminal paste shortcut, Ctrl+Shift+V: after a clipboard stream it types the clipboard it holds (D-9).
 * The shifted letter is the keysym `V`.
 */
export const PASTE_STROKES: readonly KeyStroke[] = keyStrokes({
  id: 'paste',
  label: 'Paste',
  hint: 'Send the clipboard',
  keysyms: [KEYSYM.controlLeft, KEYSYM.shiftLeft, KEYSYM.shiftedV],
});

/** Decides which keyboard events reach the device. */
export interface KeyFilter {
  /** Whether to send this press; false leaves the key to the browser. */
  down(keysym: number): boolean;
  /** Whether to send this release: only keys whose press was sent are released. */
  up(keysym: number): boolean;
}

/**
 * Every key reaches the device except while a Meta or Super key is held: then the key is the operator's own
 * shortcut (Command+V pastes through the `paste` event, D-9) and goes to the browser. A release is sent only for
 * a press that was sent, so the device never sees a key go up that never went down.
 */
export function keyFilter(): KeyFilter {
  const system = new Set<number>();
  const sent = new Set<number>();
  return {
    down: (keysym) => {
      if (SYSTEM_MODIFIERS.has(keysym)) system.add(keysym);
      if (system.size > 0) return false;
      sent.add(keysym);
      return true;
    },
    up: (keysym) => {
      system.delete(keysym);
      return sent.delete(keysym);
    },
  };
}
