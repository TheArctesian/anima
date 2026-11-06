/** X11 keysyms the Keys menu and the paste send (X11 `keysymdef.h`, as guacd reads them). */
export const KEYSYM = {
  controlLeft: 0xffe3,
  altLeft: 0xffe9,
  superLeft: 0xffeb,
  delete: 0xffff,
  escape: 0xff1b,
  tab: 0xff09,
  f4: 0xffc1,
  print: 0xff61,
  v: 0x76,
} as const;

/** The Meta and Super keys (Command on macOS): shortcuts with them belong to the operator's system, not the device. */
const SYSTEM_MODIFIERS: ReadonlySet<number> = new Set([0xffe7, 0xffe8, 0xffeb, 0xffec]);

const CONTROL: ReadonlySet<number> = new Set([0xffe3, 0xffe4]);
const SHIFT: ReadonlySet<number> = new Set([0xffe1, 0xffe2]);
const V: ReadonlySet<number> = new Set([0x56, 0x76]);
const INSERT = 0xff63;

/** One key event: 1 is a press, 0 a release (the values `sendKeyEvent` takes). */
export type KeyStroke = readonly [pressed: 0 | 1, keysym: number];

/** A key combination of the Keys menu (RA-006). */
export interface KeyCombination {
  readonly id: string;
  /** The name the menu shows, which is also the item's accessible name. */
  readonly label: string;
  /** What the combination usually does on a desktop. */
  readonly hint: string;
  /** The keys, pressed in this order and released in the reverse order. */
  readonly keysyms: readonly number[];
}

/**
 * The Keys menu of an RDP or VNC session, in menu order (RA-006): the combinations the operator's own system keeps
 * (Ctrl+Alt+Delete, Alt+Tab, the Windows key) or that a laptop lacks. "Paste" is not a combination: it sends the
 * clipboard.
 */
export const KEY_COMBINATIONS: readonly KeyCombination[] = [
  {
    id: 'ctrl-alt-delete',
    label: 'Ctrl+Alt+Delete',
    hint: 'Secure attention',
    keysyms: [KEYSYM.controlLeft, KEYSYM.altLeft, KEYSYM.delete],
  },
  { id: 'alt-tab', label: 'Alt+Tab', hint: 'Switch windows', keysyms: [KEYSYM.altLeft, KEYSYM.tab] },
  { id: 'alt-f4', label: 'Alt+F4', hint: 'Close the window', keysyms: [KEYSYM.altLeft, KEYSYM.f4] },
  { id: 'super', label: 'Windows key', hint: 'Start menu', keysyms: [KEYSYM.superLeft] },
  { id: 'print', label: 'Print Screen', hint: 'Screenshot', keysyms: [KEYSYM.print] },
  { id: 'esc', label: 'Esc', hint: 'Escape', keysyms: [KEYSYM.escape] },
  { id: 'tab', label: 'Tab', hint: 'Next field', keysyms: [KEYSYM.tab] },
];

/** The press and release sequence of `combination`: every key pressed in order, then released in reverse. */
export function keyStrokes(combination: KeyCombination): readonly KeyStroke[] {
  const presses = combination.keysyms.map((keysym): KeyStroke => [1, keysym]);
  const releases = [...combination.keysyms].reverse().map((keysym): KeyStroke => [0, keysym]);
  return [...presses, ...releases];
}

/**
 * The desktop paste shortcut, Ctrl+V, for a device that holds the keys in `held`: sent after a clipboard stream, it
 * pastes the text the device now holds (RA-005). A Ctrl the device already holds (the operator's own Ctrl+V) is
 * neither pressed again nor released, so it stays down on the device.
 */
export function pasteStrokes(held: ReadonlySet<number>): readonly KeyStroke[] {
  const holdsControl = [...CONTROL].some((key) => held.has(key));
  return keyStrokes({ id: 'paste', label: 'Paste', hint: 'Send the clipboard', keysyms: [...(holdsControl ? [] : [KEYSYM.controlLeft]), KEYSYM.v] });
}

/** The paste shortcut when the device holds no key. */
export const PASTE_STROKES: readonly KeyStroke[] = pasteStrokes(new Set());

/** Decides which keyboard events reach the device. */
export interface KeyFilter {
  /** Whether to send this press; false leaves the key to the browser. */
  down(keysym: number): boolean;
  /** Whether to send this release: only keys whose press was sent are released. */
  up(keysym: number): boolean;
  /** The keys the device holds down: every press sent and not yet released. */
  readonly held: ReadonlySet<number>;
}

/** Whether `keysym`, pressed while `held` are down, is a paste shortcut (Ctrl+V, Ctrl+Shift+V or Shift+Insert). */
function isPasteShortcut(keysym: number, held: ReadonlySet<number>): boolean {
  const any = (keys: ReadonlySet<number>) => [...keys].some((key) => held.has(key));
  if (keysym === INSERT) return any(SHIFT);
  return V.has(keysym) && any(CONTROL);
}

/**
 * Every key reaches the device except the operator's own shortcuts, which go to the browser:
 * - any key while a Meta or Super key is held (Command+V pastes through the `paste` event);
 * - with `paste` on, the paste shortcuts Ctrl+V, Ctrl+Shift+V and Shift+Insert. Left to the browser, they fire
 *   `paste` with the current system clipboard, which the view sends before it presses Ctrl+V on the device; sent
 *   to the device, they would paste whatever the device last held. With `paste` off they reach the device, which
 *   pastes its own clipboard (RA-033).
 * A release is sent only for a press that was sent, so the device never sees a key go up that never went down.
 */
export function keyFilter(options: { readonly paste: boolean }): KeyFilter {
  const system = new Set<number>();
  // The keys the device holds down: every press sent and not yet released.
  const sent = new Set<number>();
  return {
    down: (keysym) => {
      if (SYSTEM_MODIFIERS.has(keysym)) system.add(keysym);
      if (system.size > 0 || (options.paste && isPasteShortcut(keysym, sent))) return false;
      sent.add(keysym);
      return true;
    },
    up: (keysym) => {
      system.delete(keysym);
      return sent.delete(keysym);
    },
    held: sent,
  };
}
