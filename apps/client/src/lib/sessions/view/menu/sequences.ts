/** One entry of the Keys menu (RA-006): a key combination and the bytes a terminal sends for it. */
export interface KeySequence {
  readonly id: string;
  /** The name the menu shows, which is also the item's accessible name. */
  readonly label: string;
  /** What the combination usually does in a terminal. */
  readonly hint: string;
  readonly bytes: Uint8Array<ArrayBuffer>;
}

const ESCAPE = 0x1b;
const TAB = 0x09;
/** The control code of Ctrl+`letter`: the letter's code with its top bits cleared. */
const control = (letter: string) => (letter.charCodeAt(0) & 0x1f);

function sequence(id: string, label: string, hint: string, ...codes: number[]): KeySequence {
  return { id, label, hint, bytes: Uint8Array.from(codes) };
}

/**
 * The Keys menu for an SSH terminal, in menu order (RA-006, RA-007). Ctrl+Alt+Delete is not offered: a shell has no
 * such key, and the byte string a terminal would send for it is not an action on the device. "Paste" is not a
 * sequence: it sends the clipboard.
 */
export const KEY_SEQUENCES: readonly KeySequence[] = [
  sequence('ctrl-c', 'Ctrl+C', 'Interrupt', control('c')),
  sequence('ctrl-d', 'Ctrl+D', 'End of input', control('d')),
  sequence('ctrl-z', 'Ctrl+Z', 'Suspend', control('z')),
  sequence('ctrl-l', 'Ctrl+L', 'Clear the screen', control('l')),
  sequence('esc', 'Esc', 'Escape', ESCAPE),
  sequence('tab', 'Tab', 'Complete', TAB),
];
