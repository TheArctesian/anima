/** The authenticator code is six digits (OD-2); the sign-in page shows one box for each. */
export const CODE_LENGTH = 6;

/** One slot for each box: a single digit, or `''` for an empty box. */
export type Digits = readonly string[];

/** The boxes after an edit, and the box that should have focus next. */
export interface Entry {
  readonly digits: Digits;
  readonly focus: number;
}

export const EMPTY_DIGITS: Digits = Array.from({ length: CODE_LENGTH }, () => '');

/** The ASCII digits in `text`, in order. */
export function digitsIn(text: string): string {
  return text.replace(/\D/g, '');
}

function lastBox(index: number): number {
  return Math.min(Math.max(index, 0), CODE_LENGTH - 1);
}

/**
 * Typed or pasted `text` going into box `index`. Anything that is not a digit is ignored. Six or more digits fill every
 * box from the first (a pasted code or a one-time-code autofill); fewer fill the boxes from `index` on.
 */
export function enter(digits: Digits, index: number, text: string): Entry {
  const found = digitsIn(text);
  if (found === '') return { digits, focus: index };
  if (found.length >= CODE_LENGTH) return { digits: [...found.slice(0, CODE_LENGTH)], focus: CODE_LENGTH - 1 };
  const next = [...digits];
  [...found].forEach((digit, offset) => {
    if (index + offset < CODE_LENGTH) next[index + offset] = digit;
  });
  return { digits: next, focus: lastBox(index + found.length) };
}

/** Backspace in box `index`: clear it if it has a digit, otherwise move to the previous box and clear that one. */
export function backspace(digits: Digits, index: number): Entry {
  const target = digits[index] === '' ? index - 1 : index;
  if (target < 0) return { digits, focus: 0 };
  return { digits: digits.map((digit, slot) => (slot === target ? '' : digit)), focus: target };
}

/** The box an arrow key moves to from box `index`; `-1` is left, `1` is right. The ends stop. */
export function arrow(index: number, direction: -1 | 1): number {
  return lastBox(index + direction);
}

export function isComplete(digits: Digits): boolean {
  return digits.every((digit) => digit !== '');
}

export function codeOf(digits: Digits): string {
  return digits.join('');
}
