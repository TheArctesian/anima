import { describe, expect, it } from 'vitest';
import { KEY_COMBINATIONS, PASTE_STROKES, keyFilter, keyStrokes } from './keys';

const CTRL = 0xffe3;
const ALT = 0xffe9;
const SHIFT = 0xffe1;
const META = 0xffe7;

describe('Keys menu table (RA-006, D-10)', () => {
  it.each([
    ['Ctrl+C', [[1, CTRL], [1, 0x63], [0, 0x63], [0, CTRL]]],
    ['Ctrl+D', [[1, CTRL], [1, 0x64], [0, 0x64], [0, CTRL]]],
    ['Ctrl+Z', [[1, CTRL], [1, 0x7a], [0, 0x7a], [0, CTRL]]],
    ['Ctrl+L', [[1, CTRL], [1, 0x6c], [0, 0x6c], [0, CTRL]]],
    ['Esc', [[1, 0xff1b], [0, 0xff1b]]],
    ['Tab', [[1, 0xff09], [0, 0xff09]]],
    ['Ctrl+Alt+Delete', [[1, CTRL], [1, ALT], [1, 0xffff], [0, 0xffff], [0, ALT], [0, CTRL]]],
  ])('RA-006: %s presses each key in order and releases in reverse', (label, expected) => {
    const combination = KEY_COMBINATIONS.find((entry) => entry.label === label);
    expect(combination && keyStrokes(combination)).toEqual(expected);
  });

  it('D-10: the menu has exactly the combinations of D-10, in order', () => {
    expect(KEY_COMBINATIONS.map((entry) => entry.label)).toEqual(['Ctrl+C', 'Ctrl+D', 'Ctrl+Z', 'Ctrl+L', 'Esc', 'Tab', 'Ctrl+Alt+Delete']);
  });

  it('D-9: the paste keys are Ctrl+Shift+V (keysym V)', () => {
    expect(PASTE_STROKES).toEqual([
      [1, CTRL],
      [1, SHIFT],
      [1, 0x56],
      [0, 0x56],
      [0, SHIFT],
      [0, CTRL],
    ]);
  });
});

describe('keyFilter (D-8, D-9)', () => {
  it('sends every press, and a release only for a press it sent', () => {
    const filter = keyFilter();
    expect(filter.up(0x61)).toBe(false);
    expect(filter.down(0x61)).toBe(true);
    expect(filter.up(0x61)).toBe(true);
    expect(filter.up(0x61)).toBe(false);
  });

  it('D-9: while Meta is held, keys stay with the browser; after it is released they are sent again', () => {
    const filter = keyFilter();
    expect(filter.down(META)).toBe(false);
    expect(filter.down(0x76)).toBe(false);
    expect(filter.up(0x76)).toBe(false);
    expect(filter.up(META)).toBe(false);
    expect(filter.down(0x76)).toBe(true);
  });

  it('a key held before Meta is still released to the device', () => {
    const filter = keyFilter();
    filter.down(CTRL);
    filter.down(META);
    expect(filter.up(CTRL)).toBe(true);
  });
});
