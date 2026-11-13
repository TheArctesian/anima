import { describe, expect, test } from 'bun:test';
import { contrastRatio, deriveToContrast, mixColour, type HexColour } from '../src/contrast.ts';
import { aliasName, fieldsOf, numberOf, type Palette, type Token } from '../src/model.ts';
import { APPEARANCES, hexOf, palettes, ratioOf, resolveSource, ROLE_NAMES, roleToken, STATUS_SHAPES, SURFACES, tokens, type Appearance } from './support.ts';

const SERIES = [1, 2, 3, 4, 5, 6].map((n) => `chart-series-${String(n)}`);
const STATUS_NAMES = STATUS_SHAPES.map((name) => name.replace('status-', ''));

describe('palette registry (UI-023)', () => {
  test('UI-023: Graphite is the default palette and Nord is registered second, each with its licence line', () => {
    expect(palettes).toEqual([
      { id: 'graphite', name: 'Graphite', licence: 'Anima Graphite, part of Anima', default: true, file: 'graphite.tokens.json' },
      { id: 'nord', name: 'Nord', licence: 'Nord by Arctic Ice Studio, MIT', default: false, file: 'nord.tokens.json' },
    ]);
  });

  test('UI-023: exactly one palette is the default', () => {
    expect(palettes.filter((palette) => palette.default)).toHaveLength(1);
  });

  test('UI-023: every registered palette has roles in the token source', () => {
    for (const palette of palettes) {
      expect(tokens.some((token) => token.palette === palette.id)).toBe(true);
    }
  });
});

describe('UI-024: every registered palette fills every role for dark and light', () => {
  for (const palette of palettes) {
    for (const appearance of APPEARANCES) {
      test(`UI-024: ${palette.id} ${appearance} has a value for every role, and each resolves to a hex`, () => {
        for (const role of ROLE_NAMES) {
          expect(roleToken(palette.id, role)).toBeDefined();
          // A scrim is a mix over transparent, so it has no hex of its own; its mix is checked by the golden tests.
          if (role !== 'scrim') expect(hexOf(palette.id, role, appearance)).toMatch(/^#[0-9A-F]{6}$/);
        }
      });
    }
    test(`UI-024: ${palette.id} names only its own primitives and roles`, () => {
      const aliases = (value: unknown): string[] => {
        if (typeof value === 'string') return [...value.matchAll(/\{([^{}]+)\}/g)].map((match) => match[1] ?? '');
        if (typeof value === 'object' && value !== null) return Object.values(value).flatMap(aliases);
        return [];
      };
      for (const token of tokens.filter((candidate) => candidate.palette === palette.id)) {
        for (const alias of [...aliases(token.source), ...aliases(token.light?.source)]) {
          expect(alias.startsWith(`color.${palette.id}.`) || alias.startsWith(`color.role.${palette.id}.`)).toBe(true);
        }
      }
    });
  }
});

describe('UI-002, UI-004, UI-005: contrast rules per palette and appearance', () => {
  for (const palette of palettes) {
    for (const appearance of APPEARANCES) {
      describe(`${palette.id} ${appearance}`, () => {
        textRules(palette, appearance);
        statusRules(palette, appearance);
      });
    }
  }
});

function textRules(palette: Palette, appearance: Appearance): void {
        for (const surface of SURFACES) {
          test(`UI-002: text and text-secondary are at least 4.5:1 on ${surface}`, () => {
            expect(ratioOf(palette.id, 'text', surface, appearance)).toBeGreaterThanOrEqual(4.5);
            expect(ratioOf(palette.id, 'text-secondary', surface, appearance)).toBeGreaterThanOrEqual(4.5);
          });
        }

        test('UI-002: text-tertiary is 3:1 on surface-overlay (dark) or 4.5:1 on surface-raised (light)', () => {
          if (appearance === 'dark') {
            expect(ratioOf(palette.id, 'text-tertiary', 'surface-overlay', 'dark')).toBeGreaterThanOrEqual(3);
          } else {
            expect(ratioOf(palette.id, 'text-tertiary', 'surface-raised', 'light')).toBeGreaterThanOrEqual(4.5);
          }
        });

}

function statusRules(palette: Palette, appearance: Appearance): void {
        test('UI-004: on-accent is at least 4.5:1 on accent', () => {
          expect(ratioOf(palette.id, 'on-accent', 'accent', appearance)).toBeGreaterThanOrEqual(4.5);
        });

        test('UI-004: on-danger is at least 4.5:1 on danger, in both appearances (fix round 1)', () => {
          expect(ratioOf(palette.id, 'on-danger', 'danger', appearance)).toBeGreaterThanOrEqual(4.5);
        });

        test('5.1.2: the accent is at least 3:1 on surface-raised and surface-sunken (focus ring)', () => {
          expect(ratioOf(palette.id, 'accent', 'surface-raised', appearance)).toBeGreaterThanOrEqual(3);
          expect(ratioOf(palette.id, 'accent', 'surface-sunken', appearance)).toBeGreaterThanOrEqual(3);
        });

        for (const shape of STATUS_SHAPES) {
          test(`UI-005: ${shape} is at least 3:1 on surface-raised and surface-sunken`, () => {
            expect(ratioOf(palette.id, shape, 'surface-raised', appearance)).toBeGreaterThanOrEqual(3);
            expect(ratioOf(palette.id, shape, 'surface-sunken', appearance)).toBeGreaterThanOrEqual(3);
          });
        }

        for (const name of STATUS_NAMES) {
          test(`UI-005: status-${name}-text is at least 4.5:1 on its own tint`, () => {
            const text = hexOf(palette.id, `status-${name}-text`, appearance);
            const tint = hexOf(palette.id, `status-${name}-tint`, appearance);
            expect(contrastRatio(text, tint)).toBeGreaterThanOrEqual(4.5);
          });
        }

        test('WCAG 1.4.11: border-strong is at least 3:1 on surface-raised and surface-sunken, so an input is identified', () => {
          expect(ratioOf(palette.id, 'border-strong', 'surface-raised', appearance)).toBeGreaterThanOrEqual(3);
          expect(ratioOf(palette.id, 'border-strong', 'surface-sunken', appearance)).toBeGreaterThanOrEqual(3);
        });

        test('UI-002: text-tertiary differs from text-secondary, and is the lightest light tint that keeps 4.5:1 on surface-raised', () => {
          if (appearance !== 'light') return;
          const tertiary = hexOf(palette.id, 'text-tertiary', 'light');
          expect(tertiary).not.toBe(hexOf(palette.id, 'text-secondary', 'light'));
          expect(ratioOf(palette.id, 'text-tertiary', 'surface-raised', 'light')).toBeGreaterThanOrEqual(4.5);
          const secondary = hexOf(palette.id, 'text-secondary', 'light');
          const raised = hexOf(palette.id, 'surface-raised', 'light');
          const steps = Array.from({ length: 101 }, (_, step) => mixColour(secondary, raised, step / 100));
          const at = steps.indexOf(tertiary);
          expect(at).toBeGreaterThan(0);
          expect(contrastRatio(steps[at + 1] ?? raised, raised)).toBeLessThan(4.5);
        });

        test('UI-005: the light accent is at least 4.5:1 on every light surface tier', () => {
          if (appearance !== 'light') return;
          for (const surface of SURFACES) {
            expect(ratioOf(palette.id, 'accent', surface, 'light')).toBeGreaterThanOrEqual(4.5);
          }
        });

        // The chart series rule (K5, MET-004) is checked on the light surfaces, as before.
        if (appearance === 'light') {
          for (const series of SERIES) {
            test(`K5: ${series} is at least 3:1 on surface-raised and surface-sunken (light)`, () => {
              expect(ratioOf(palette.id, series, 'surface-raised', 'light')).toBeGreaterThanOrEqual(3);
              expect(ratioOf(palette.id, series, 'surface-sunken', 'light')).toBeGreaterThanOrEqual(3);
            });
          }
        }
}

describe('derived values record their rule (UI-005, D-27)', () => {
  test('UI-005: nord-10-deep is nord-10 mixed toward nord-0 until 4.5:1 on nord-5 (the light accent)', () => {
    const value = (name: string): HexColour => tokens.find((token) => token.name === name)?.value as HexColour;
    const derived = deriveToContrast(value('nord-10'), value('nord-0'), value('nord-5'), 4.5);
    expect(value('nord-10-deep')).toBe(derived.colour);
  });

  test('UI-005: each dark status shape is lifted to 3:1 on the dark surface-raised (nord-1)', () => {
    const value = (name: string): HexColour => tokens.find((token) => token.name === name)?.value as HexColour;
    for (const n of [11, 12, 13, 14, 15]) {
      const lifted = deriveToContrast(value(`nord-${String(n)}`), value('nord-6'), value('nord-1'), 3);
      expect(value(`nord-${String(n)}-lifted`)).toBe(lifted.colour);
    }
  });
});

describe('the Graphite values (operator brief, 2026-11-12)', () => {
  const table: [string, Appearance, string][] = [
    ['surface-sunken', 'dark', '#16181B'], ['surface-base', 'dark', '#1B1D21'], ['surface-raised', 'dark', '#23262B'],
    ['surface-overlay', 'dark', '#2C3036'], ['surface-hover', 'dark', '#2C3036'], ['border-subtle', 'dark', '#2C3036'],
    ['text', 'dark', '#E6E7E9'], ['text-secondary', 'dark', '#A4A8B0'], ['text-tertiary', 'dark', '#7E838C'],
    ['surface-sunken', 'light', '#EEEFF1'], ['surface-base', 'light', '#F6F7F8'], ['surface-raised', 'light', '#FFFFFF'],
    ['surface-overlay', 'light', '#FFFFFF'], ['surface-hover', 'light', '#EEEFF1'], ['border-subtle', 'light', '#E3E5E8'],
    ['text', 'light', '#1D2024'], ['text-secondary', 'light', '#5A5F68'], ['on-accent', 'dark', '#16181B'], ['on-accent', 'light', '#FFFFFF'],
    ['accent', 'dark', '#88C0D0'],
  ];
  for (const [role, appearance, hex] of table) {
    test(`Graphite ${appearance} ${role} is ${hex}`, () => {
      expect(hexOf('graphite', role, appearance)).toBe(hex as HexColour);
    });
  }

  test('WCAG 1.4.11: Graphite border-strong is derived to 3:1, as the operator values (#C3C7CD, #41464E) are below it', () => {
    expect(ratioOf('graphite', 'border-strong', 'surface-raised', 'light')).toBeGreaterThanOrEqual(3);
    expect(ratioOf('graphite', 'border-strong', 'surface-sunken', 'light')).toBeGreaterThanOrEqual(3);
    expect(ratioOf('graphite', 'border-strong', 'surface-raised', 'dark')).toBeGreaterThanOrEqual(3);
    expect(hexOf('graphite', 'border-strong', 'light')).not.toBe('#C3C7CD');
    expect(hexOf('graphite', 'border-strong', 'dark')).not.toBe('#41464E');
  });

  test('UI-005: the Graphite light accent is the chosen accent-deep, #2E6E80, not a linear mix', () => {
    expect(hexOf('graphite', 'accent', 'light')).toBe('#2E6E80');
  });

  test('UI-005: Graphite light text-tertiary is derived to 4.5:1 on surface-raised, as the operator value is 4.44:1', () => {
    expect(ratioOf('graphite', 'text-tertiary', 'surface-raised', 'light')).toBeGreaterThanOrEqual(4.5);
    expect(hexOf('graphite', 'text-tertiary', 'light')).not.toBe('#737881');
  });

  test('UI-023: Graphite and Nord roles differ only in their values, not in their names', () => {
    expect(ROLE_NAMES.every((role) => roleToken('graphite', role).name === roleToken('nord', role).name)).toBe(true);
  });
});

describe('the Nord surface values (5.1.1, 5.1.2)', () => {
  test('UI-005: Nord light surfaces are nord-5, nord-6, nord-6 mixed 60 % toward white, and white', () => {
    expect(['surface-sunken', 'surface-base', 'surface-raised', 'surface-overlay'].map((role) => hexOf('nord', role, 'light'))).toEqual([
      '#E5E9F0', '#ECEFF4', '#F7F9FB', '#FFFFFF',
    ]);
  });

  test('UI-005: Nord dark surface-sunken is nord-0 mixed 15 % toward black', () => {
    expect(hexOf('nord', 'surface-sunken', 'dark')).toBe('#272C36');
  });

  test('UI-002: Nord text-secondary is nord-4 mixed 25 % toward nord-3 (dark)', () => {
    expect(hexOf('nord', 'text-secondary', 'dark')).toBe('#B5BCC9');
  });
});

const MATERIALS = ['chrome', 'panel', 'overlay'] as const;
const MINIMUM = 4.5;

/** The content colours of a palette that can show through a material: surfaces, text, accent, status and chart colours. */
const BACKDROP_ROLES = [
  'surface-sunken', 'surface-base', 'surface-raised', 'surface-overlay', 'surface-hover', 'surface-selected',
  'text', 'text-secondary', 'text-tertiary', 'accent', 'danger',
  ...['up', 'down', 'degraded', 'warning', 'maintenance', 'unknown'].map((name) => `status-${name}`),
  ...[1, 2, 3, 4, 5, 6].map((n) => `chart-series-${String(n)}`),
];

function materialToken(name: string): Token {
  const token = tokens.find((candidate) => candidate.name === `material-${name}`);
  if (!token) throw new Error(`no material-${name}`);
  return token;
}

/** The white (or other highlight) a role mixes over transparent, and how much of it, in one appearance. */
function highlightOf(palette: string, role: string, appearance: Appearance): { colour: HexColour; fraction: number } {
  const token = roleToken(palette, role);
  const source = appearance === 'light' ? (token.light?.source ?? token.source) : token.source;
  const parts = fieldsOf(source);
  return { colour: resolveSource(parts['color'], appearance), fraction: numberOf(parts['percent']) / 100 };
}

/**
 * The lowest contrast of `text` on a material, over every content colour that can sit behind it (MAT-005). The material
 * is its base at its opacity over the backdrop, plus the sheen at its strongest (the top edge).
 */
function worstMaterialContrast(palette: string, material: string, appearance: Appearance): { ratio: number; backdrop: string } {
  const token = materialToken(material);
  const parts = fieldsOf(token.value);
  const opacity = numberOf(parts['opacityPercent']) / 100;
  const base = hexOf(palette, aliasName(fieldsOf(token.source)['base']) ?? '', appearance);
  const sheen = highlightOf(palette, 'material-sheen', appearance);
  const text = hexOf(palette, 'text', appearance);
  const results = BACKDROP_ROLES.map((role) => {
    const frosted = mixColour(hexOf(palette, role, appearance), base, opacity);
    return { backdrop: role, ratio: contrastRatio(text, mixColour(frosted, sheen.colour, sheen.fraction)) };
  });
  return results.reduce((worst, next) => (next.ratio < worst.ratio ? next : worst));
}

describe('MAT-005: text on a material keeps 4.5:1 over the worst backdrop of every palette', () => {
  for (const palette of palettes) {
    for (const appearance of APPEARANCES) {
      for (const material of MATERIALS) {
        test(`MAT-005: ${palette.id} ${appearance} material-${material}`, () => {
          const { ratio, backdrop } = worstMaterialContrast(palette.id, material, appearance);
          expect(ratio, `worst backdrop ${backdrop}`).toBeGreaterThanOrEqual(MINIMUM);
        });
      }
    }
  }
});

describe('MAT-001, MAT-010: the shape of the three materials', () => {
  const blur = (name: string): number => numberOf(String(fieldsOf(materialToken(name).value)['blur']).replace('px', ''));

  test('MAT-001: the overlay (which holds the largest surface, the modal) blurs most, then the chrome, then the panel', () => {
    expect(blur('overlay')).toBeGreaterThan(blur('chrome'));
    expect(blur('chrome')).toBeGreaterThan(blur('panel'));
  });

  test('MAT-010: a material is more translucent than the old 82 to 92 % and never below half', () => {
    for (const name of MATERIALS) {
      const opacity = numberOf(fieldsOf(materialToken(name).value)['opacityPercent']);
      expect(opacity).toBeGreaterThanOrEqual(50);
      expect(opacity).toBeLessThanOrEqual(90);
    }
  });

  test('MAT-010: the sheen stays at a few percent and the edge is stronger on a light theme', () => {
    for (const palette of palettes) {
      for (const appearance of APPEARANCES) {
        expect(highlightOf(palette.id, 'material-sheen', appearance).fraction).toBeLessThanOrEqual(0.06);
      }
      expect(highlightOf(palette.id, 'material-edge', 'light').fraction).toBeGreaterThan(highlightOf(palette.id, 'material-edge', 'dark').fraction);
    }
  });
});
