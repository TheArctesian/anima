import { describe, expect, test } from 'bun:test';
import {
  contrastRatio,
  deriveToContrast,
  largestMixToContrast,
  MalformedColourError,
  mixColour,
  UnreachableContrastError,
} from '../src/contrast.ts';

const NORD_0 = '#2E3440';
const NORD_4 = '#D8DEE9';
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

describe('derived light primitives (UI-005, D-27, R-086)', () => {
  for (const [name, colour] of Object.entries(AURORA)) {
    test(`UI-005: nord-${name} derives to 3:1 on nord-4 and one percent less does not`, () => {
      const derived = deriveToContrast(colour, NORD_0, NORD_4, 3);
      expect(contrastRatio(derived.colour, NORD_4)).toBeGreaterThanOrEqual(3);
      if (derived.mixPercent > 0) {
        const lessMixed = mixColour(colour, NORD_0, (derived.mixPercent - 1) / 100);
        expect(contrastRatio(lessMixed, NORD_4)).toBeLessThan(3);
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

describe('largest derived mix (UI-002: a tint stays apart from its base)', () => {
  const GREY = '#5A5F68';
  const WHITE = '#FFFFFF';

  test('UI-002: the largest mix keeps the minimum, and one whole percent more falls below it', () => {
    const derived = largestMixToContrast(GREY, WHITE, WHITE, 4.5);
    expect(contrastRatio(derived.colour, WHITE)).toBeGreaterThanOrEqual(4.5);
    expect(contrastRatio(mixColour(GREY, WHITE, (derived.mixPercent + 1) / 100), WHITE)).toBeLessThan(4.5);
  });

  test('UI-002: the largest mix is the lightest passing step, so it is not the colour itself', () => {
    const derived = largestMixToContrast(GREY, WHITE, WHITE, 4.5);
    expect(derived.mixPercent).toBeGreaterThan(0);
    expect(derived.colour).not.toBe(GREY);
  });

  test('D-27: a colour that fails at 0 % has no passing mix, a typed error', () => {
    expect(() => largestMixToContrast('#FFFFFF', WHITE, WHITE, 4.5)).toThrow(UnreachableContrastError);
  });

  test('D-27: where every step passes the largest mix is 100 %', () => {
    expect(largestMixToContrast('#000000', '#FFFFFF', '#000000', 0).mixPercent).toBe(100);
  });
});
