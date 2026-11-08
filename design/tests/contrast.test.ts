import { describe, expect, test } from 'bun:test';
import {
  contrastRatio,
  deriveToContrast,
  MalformedColourError,
  mixColour,
  UnreachableContrastError,
} from '../src/contrast.ts';

const NORD_0 = '#2E3440';
const NORD_6 = '#ECEFF4';
const AURORA = { 11: '#BF616A', 12: '#D08770', 13: '#EBCB8B', 14: '#A3BE8C', 15: '#B48EAD' } as const;

describe('contrast ratio (UI-005, MAT-005)', () => {
  test('UI-005: black on white is 21:1', () => {
    expect(contrastRatio('#000000', '#FFFFFF')).toBeCloseTo(21, 9);
  });
  test('UI-005: a colour on itself is 1:1', () => {
    expect(contrastRatio(NORD_6, NORD_6)).toBe(1);
  });
  test('UI-005: the order of the colours does not matter', () => {
    expect(contrastRatio(NORD_0, NORD_6)).toBe(contrastRatio(NORD_6, NORD_0));
  });
  test('UI-005: a malformed colour is refused, never NaN', () => {
    expect(() => contrastRatio('#FFF', NORD_6)).toThrow(MalformedColourError);
    expect(() => contrastRatio('red', NORD_6)).toThrow(MalformedColourError);
    expect(() => contrastRatio('#GGGGGG', NORD_6)).toThrow(MalformedColourError);
  });
});

describe('colour mix', () => {
  test('D-27: 0 % keeps the colour and 100 % reaches the target', () => {
    expect(mixColour(AURORA[13], NORD_0, 0)).toBe(AURORA[13]);
    expect(mixColour(AURORA[13], NORD_0, 1)).toBe(NORD_0);
  });
  test('D-27: 50 % of black toward white is the rounded mid grey', () => {
    expect(mixColour('#000000', '#FFFFFF', 0.5)).toBe('#808080');
  });
});

describe('derived light primitives (UI-005, D-27)', () => {
  for (const [name, colour] of Object.entries(AURORA)) {
    test(`UI-005: nord-${name} derives to 3:1 on nord-6 and one percent less does not`, () => {
      const derived = deriveToContrast(colour, NORD_0, NORD_6, 3);
      expect(contrastRatio(derived.colour, NORD_6)).toBeGreaterThanOrEqual(3);
      if (derived.mixPercent > 0) {
        const lessMixed = mixColour(colour, NORD_0, (derived.mixPercent - 1) / 100);
        expect(contrastRatio(lessMixed, NORD_6)).toBeLessThan(3);
      }
    });
  }
  test('D-27: a colour that already passes is kept at 0 %', () => {
    expect(deriveToContrast('#000000', NORD_0, NORD_6, 3)).toEqual({ colour: '#000000', mixPercent: 0 });
  });
  test('D-27: an unreachable minimum is a typed error', () => {
    expect(() => deriveToContrast(AURORA[13], NORD_0, NORD_6, 30)).toThrow(UnreachableContrastError);
  });
});
