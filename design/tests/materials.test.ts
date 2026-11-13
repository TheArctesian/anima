import { describe, expect, test } from 'bun:test';
import { contrastRatio, mixColour, type HexColour } from '../src/contrast.ts';
import { aliasName, fieldsOf, numberOf, type Token } from '../src/model.ts';
import { APPEARANCES, hexOf, palettes, resolveSource, roleToken, tokens, type Appearance } from './support.ts';

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
