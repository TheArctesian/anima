import { describe, expect, it } from 'vitest';
import { KEY_COMBINATIONS, PASTE_STROKES, keyFilter, keyStrokes, pasteStrokes } from './keys';

const CTRL = 0xffe3;
const ALT = 0xffe9;
const SHIFT = 0xffe1;
const META = 0xffe7;
const V = 0x76;

describe('Keys menu table (RA-006)', () => {
  it.each([
    ['Ctrl+Alt+Delete', [[1, CTRL], [1, ALT], [1, 0xffff], [0, 0xffff], [0, ALT], [0, CTRL]]],
    ['Alt+Tab', [[1, ALT], [1, 0xff09], [0, 0xff09], [0, ALT]]],
    ['Alt+F4', [[1, ALT], [1, 0xffc1], [0, 0xffc1], [0, ALT]]],
    ['Windows key', [[1, 0xffeb], [0, 0xffeb]]],
    ['Print Screen', [[1, 0xff61], [0, 0xff61]]],
    ['Esc', [[1, 0xff1b], [0, 0xff1b]]],
    ['Tab', [[1, 0xff09], [0, 0xff09]]],
  ])('RA-006: %s presses each key in order and releases in reverse', (label, expected) => {
    const combination = KEY_COMBINATIONS.find((entry) => entry.label === label);
    expect(combination && keyStrokes(combination)).toEqual(expected);
  });

  it('RA-006: the menu starts with Ctrl+Alt+Delete and has each combination once', () => {
    expect(KEY_COMBINATIONS.map((entry) => entry.label)).toEqual(['Ctrl+Alt+Delete', 'Alt+Tab', 'Alt+F4', 'Windows key', 'Print Screen', 'Esc', 'Tab']);
    expect(new Set(KEY_COMBINATIONS.map((entry) => entry.id)).size).toBe(KEY_COMBINATIONS.length);
  });

  it('RA-005: the paste keys are Ctrl+V', () => {
    expect(PASTE_STROKES).toEqual([
      [1, CTRL],
      [1, V],
      [0, V],
      [0, CTRL],
    ]);
  });
});

it.each([
  ['left Ctrl', new Set([CTRL]), [[1, V], [0, V]]],
  ['right Ctrl and Shift', new Set([0xffe4, SHIFT]), [[1, V], [0, V]]],
  ['Shift', new Set([0xffe2]), [[1, CTRL], [1, V], [0, V], [0, CTRL]]],
])('RA-005: with %s held on the device, the paste keys leave it down', (_name, held, expected) => {
  expect(pasteStrokes(held)).toEqual(expected);
});

describe('keyFilter (D-8)', () => {
  it.each([true, false])('paste %s: sends every press, and a release only for a press it sent', (paste) => {
    const filter = keyFilter({ paste });
    expect(filter.up(0x61)).toBe(false);
    expect(filter.down(0x61)).toBe(true);
    expect(filter.held).toEqual(new Set([0x61]));
    expect(filter.up(0x61)).toBe(true);
    expect(filter.up(0x61)).toBe(false);
    expect(filter.held).toEqual(new Set());
  });

  it.each([true, false])('paste %s: while Meta is held, keys stay with the browser; after it is released they are sent again', (paste) => {
    const filter = keyFilter({ paste });
    expect(filter.down(META)).toBe(false);
    expect(filter.down(0x76)).toBe(false);
    expect(filter.up(0x76)).toBe(false);
    expect(filter.up(META)).toBe(false);
    expect(filter.down(0x76)).toBe(true);
  });

  it.each([0xffe8, 0xffeb, 0xffec])('the system modifier %s keeps keys with the browser too', (modifier) => {
    const filter = keyFilter({ paste: true });
    expect(filter.down(modifier)).toBe(false);
    expect(filter.down(0x61)).toBe(false);
  });

  it('a key held before Meta is still released to the device', () => {
    const filter = keyFilter({ paste: true });
    filter.down(CTRL);
    filter.down(META);
    expect(filter.up(CTRL)).toBe(true);
  });

  it.each([
    ['Ctrl+V', [CTRL], 0x76],
    ['right Ctrl+V', [0xffe4], 0x76],
    ['Ctrl+Shift+V', [CTRL, SHIFT], 0x56],
    ['Ctrl+Shift+v', [CTRL, SHIFT], 0x76],
    ['Shift+Insert', [SHIFT], 0xff63],
    ['right Shift+Insert', [0xffe2], 0xff63],
  ])('RA-005: with paste on, %s goes to the browser, which pastes the system clipboard', (_name, modifiers, last) => {
    const filter = keyFilter({ paste: true });
    for (const modifier of modifiers) expect(filter.down(modifier)).toBe(true);
    expect(filter.down(last)).toBe(false);
    expect(filter.up(last)).toBe(false);
  });
});

describe('keyFilter paste shortcuts (RA-005, RA-033)', () => {
  it.each([
    ['Ctrl+V', [CTRL], 0x76],
    ['Ctrl+Shift+V', [CTRL, SHIFT], 0x56],
    ['Shift+Insert', [SHIFT], 0xff63],
  ])('RA-033: with paste off, %s reaches the device, which pastes its own clipboard', (_name, modifiers, last) => {
    const filter = keyFilter({ paste: false });
    for (const modifier of modifiers) filter.down(modifier);
    expect(filter.down(last)).toBe(true);
  });

  it.each([
    ['Shift+V', [SHIFT], 0x56],
    ['V', [], 0x76],
    ['Insert', [], 0xff63],
    ['Ctrl+Insert', [CTRL], 0xff63],
  ])('%s still reaches the device with paste on', (_name, modifiers, last) => {
    const filter = keyFilter({ paste: true });
    for (const modifier of modifiers) filter.down(modifier);
    expect(filter.down(last)).toBe(true);
  });
});
