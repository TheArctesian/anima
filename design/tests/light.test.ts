import { describe, expect, test } from 'bun:test';
import { resolve } from 'node:path';
import { deriveToContrast, type HexColour } from '../src/contrast.ts';
import { renderCss } from '../src/css.ts';
import { loadTokens } from '../src/load.ts';
import { renderReference } from '../src/reference.ts';
import { isSemanticColour } from '../src/model.ts';
import { palettes, tokens } from './support.ts';

const reference = (): string => renderReference(tokens);
const css = (): string => renderCss(tokens, palettes);

/** The declarations of the first block that opens with `opening` after `from` (an index into the CSS text). */
function body(text: string, opening: string, from = 0): string[] {
  const start = text.indexOf(`${opening} {`, from);
  if (start < 0) return [];
  const lineStart = text.lastIndexOf('\n', start) + 1;
  const close = text.indexOf(`\n${text.slice(lineStart, start)}}`, start);
  return text.slice(start + opening.length + 2, close).split('\n').map((line) => line.trim()).filter((line) => line !== '');
}

/** The light theme block of one palette: the one after its label, under data-theme="light". */
function paletteLight(text: string, label: string): string[] {
  const at = text.indexOf(`/* Palette ${label} (UI-023)`);
  const selector = label === 'Graphite' ? ':root[data-theme="light"]' : ':root[data-palette="nord"][data-theme="light"]';
  return body(text, selector, at);
}

describe('deep primitives (UI-005, D-27, R-086)', () => {
  const find = (name: string): HexColour => tokens.find((token) => token.name === name && token.palette === undefined)?.value as HexColour;

  for (const n of [11, 12, 13, 14, 15]) {
    test(`UI-005: nord-${String(n)}-deep is nord-${String(n)} mixed toward nord-0 until 3:1 on nord-4`, () => {
      const derived = deriveToContrast(find(`nord-${String(n)}`), find('nord-0'), find('nord-4'), 3);
      expect(find(`nord-${String(n)}-deep`)).toBe(derived.colour);
    });
  }
});

describe('light theme values (TOK-011, UI-005)', () => {
  test('TOK-011: only semantic colours have a light value', () => {
    for (const token of tokens.filter((candidate) => candidate.light !== undefined)) expect(isSemanticColour(token)).toBe(true);
  });

  test('UI-005: components never see a primitive; the light block of each palette names semantic tokens only', () => {
    const text = css();
    for (const label of ['Graphite', 'Nord']) {
      const light = paletteLight(text, label);
      expect(light.length).toBeGreaterThan(0);
      expect(light.some((line) => /^--(nord|graphite)-/.test(line))).toBe(false);
    }
  });

  test('UI-023: the Graphite light status shape is its deep aurora value, and the Nord one is its nord-deep value', () => {
    const text = css();
    expect(paletteLight(text, 'Graphite')).toContain('--status-down: var(--graphite-aurora-11-deep);');
    expect(paletteLight(text, 'Nord')).toContain('--status-down: var(--nord-11-deep);');
  });

  test('UI-005: the Nord light accent is the derived nord-10-deep', () => {
    expect(paletteLight(css(), 'Nord')).toContain('--accent: var(--nord-10-deep);');
  });
});

describe('palette selectors (UI-023)', () => {
  test('UI-023: Graphite sits on :root and under data-palette="graphite"; Nord is selected by data-palette="nord"', () => {
    const text = css();
    expect(text).toContain(':root, :root[data-palette="graphite"] {');
    expect(text).toContain(':root[data-palette="nord"] {');
    expect(text).toContain(':root[data-palette="nord"][data-theme="light"] {');
    expect(text).toContain('  :root[data-palette="nord"][data-theme="system"] {');
  });

  test('UI-023: the Nord blocks come after the Graphite blocks, so the chosen palette wins at each appearance', () => {
    const text = css();
    expect(text.indexOf(':root[data-palette="nord"] {')).toBeGreaterThan(text.indexOf(':root[data-theme="light"] {'));
  });

  test('MAT-006: the preference blocks match every palette and appearance selector, so they come last', () => {
    const text = css();
    const preference = text.indexOf('@media (prefers-contrast: more)');
    expect(preference).toBeGreaterThan(text.indexOf(':root[data-palette="nord"][data-theme="system"]'));
    expect(text.slice(preference)).toContain(':root, :root[data-theme], :root[data-palette], :root[data-palette][data-theme] {');
  });
});

describe('surface-hover (CR-10, 5.1.2)', () => {
  test('5.1.2: the dark surface-hover is nord-2 (Nord) and graphite-dark-overlay (Graphite)', () => {
    expect(tokens.find((token) => token.name === 'surface-hover' && token.palette === 'nord')?.source).toBe('{color.nord.2}');
    expect(tokens.find((token) => token.name === 'surface-hover' && token.palette === 'graphite')?.source).toBe('{color.graphite.dark-overlay}');
  });

  test('5.1.2: the light surface-hover is nord-5 (Nord) and graphite-light-sunken (Graphite)', () => {
    expect(tokens.find((token) => token.name === 'surface-hover' && token.palette === 'nord')?.light?.source).toBe('{color.nord.5}');
    expect(tokens.find((token) => token.name === 'surface-hover' && token.palette === 'graphite')?.light?.source).toBe('{color.graphite.light-sunken}');
  });
});

describe('reference page (UI-005, UI-023)', () => {
  test('UI-005: a role shows its dark value, then its light value on a second line', () => {
    expect(reference()).toContain('`--surface-raised: var(--nord-1)` = `#3B4252`<br>light: `--surface-raised: var(--nord-6-raised)` = `#F7F9FB`');
  });

  test('UI-005: a light mix shows as its color-mix, as a tint does', () => {
    expect(reference()).toContain(
      '`--accent-hover: color-mix(in srgb, var(--nord-6) 15%, var(--nord-8))`<br>light: `--accent-hover: color-mix(in srgb, var(--nord-0) 12%, var(--accent))`',
    );
  });

  test('UI-023: the reference page documents both palettes', () => {
    expect(reference()).toContain('## color.role.graphite');
    expect(reference()).toContain('## color.role.nord');
    expect(reference()).toContain('Graphite is the default');
  });

  test('5.6: the elevation note states the rule for cards: surface contrast, no border, no shadow', () => {
    expect(reference()).toContain('`elevation-1` is surface contrast, no border and no shadow');
  });
});

describe('the token source loads with the palette files (UI-023)', () => {
  test('UI-023: palette tokens are primitives and roles are palette-scoped', async () => {
    const loaded = await loadTokens(resolve(import.meta.dir, '..', 'tokens'));
    expect(loaded.find((token) => token.name === 'nord-0')?.palette).toBeUndefined();
    expect(loaded.find((token) => token.name === 'surface-raised' && token.palette === 'graphite')).toBeDefined();
  });
});
