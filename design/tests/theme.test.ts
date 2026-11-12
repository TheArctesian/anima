import { afterEach, describe, expect, test } from 'bun:test';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { renderCss } from '../src/css.ts';
import { loadTokens } from '../src/load.ts';
import { TokenSourceError, type ModeValue, type Palette, type Token } from '../src/model.ts';

/** No palette: the synthetic tokens are shared, so only the shared blocks are under test. */
const NO_PALETTES: Palette[] = [];
const PREFERENCE = ':root, :root[data-theme], :root[data-palette], :root[data-palette][data-theme]';

function colour(name: string, source: string, value: string, light?: ModeValue): Token {
  return { name, path: `color.${name}`, group: 'color', type: 'color', value, source, description: '', reduced: undefined, light, palette: undefined };
}

const PRIMITIVES: Token[] = [
  { ...colour('nord-0', '#2E3440', '#2E3440'), path: 'color.nord.0', group: 'color.nord' },
  { ...colour('nord-5', '#E5E9F0', '#E5E9F0'), path: 'color.nord.5', group: 'color.nord' },
];
const MATERIAL: Token = {
  name: 'material-panel',
  path: 'material.panel',
  group: 'material',
  type: 'material',
  value: { base: '#2E3440', opacityPercent: 90, blur: '12px', saturatePercent: 100 },
  source: { base: '{color.surface-base}', opacityPercent: 90, blur: '12px', saturatePercent: 100 },
  description: '',
  reduced: undefined,
  light: undefined,
  palette: undefined,
};

/** A synthetic token set: `surface-base` and `border-strong` change in the light theme, `text` does not. */
const THEMED: Token[] = [
  ...PRIMITIVES,
  colour('surface-base', '{color.nord.0}', '#2E3440', { source: '{color.nord.5}', value: '#E5E9F0' }),
  colour('border-strong', '{color.nord.5}', '#E5E9F0', { source: '{color.nord.0}', value: '#2E3440' }),
  colour('text', '{color.nord.5}', '#E5E9F0'),
  MATERIAL,
];
const DARK_ONLY: Token[] = THEMED.map((token) => ({ ...token, light: undefined }));

/** The declarations inside the first block that opens with `opening`. */
function body(css: string, opening: string): string[] {
  const start = css.indexOf(`${opening} {`);
  if (start < 0) return [];
  const text = css.slice(start + opening.length + 2, css.indexOf('}', start));
  return text.split('\n').map((line) => line.trim()).filter((line) => line !== '');
}

describe('light theme blocks (UI-005, K5)', () => {
  test('UI-005: light values give the light selector and the System-on-a-light-OS selector the same declarations', () => {
    const css = renderCss(THEMED, NO_PALETTES);
    const light = body(css, ':root[data-theme="light"]');
    expect(light).toEqual(['--surface-base: var(--nord-5);', '--border-strong: var(--nord-0);']);
    expect(css).toContain('@media (prefers-color-scheme: light) {\n  :root[data-theme="system"] {');
    expect(body(css, '  :root[data-theme="system"]')).toEqual(light);
  });

  test('UI-005: without a light value there is no theme block and the preference blocks keep their plain selectors', () => {
    const css = renderCss(DARK_ONLY, NO_PALETTES);
    expect(css).not.toContain(':root[data-theme="light"]');
    expect(css).not.toContain('prefers-color-scheme');
    expect(css).toContain(`@media (prefers-contrast: more) {\n  ${PREFERENCE} {`);
  });

  test('MAT-006: increased contrast and reduced transparency come after the light blocks and match their specificity', () => {
    const css = renderCss(THEMED, NO_PALETTES);
    const system = css.indexOf(':root[data-theme="system"]');
    for (const query of ['prefers-reduced-transparency: reduce', 'prefers-contrast: more']) {
      const at = css.indexOf(`@media (${query}) {\n  ${PREFERENCE} {`);
      expect(at).toBeGreaterThan(system);
    }
    expect(body(css, `  ${PREFERENCE}`)).toContain('--material-panel-bg: var(--surface-base);');
  });

  test('UI-005: the dark :root block is the same with and without light values', () => {
    expect(body(renderCss(THEMED, NO_PALETTES), ':root')).toEqual(body(renderCss(DARK_ONLY, NO_PALETTES), ':root'));
  });
});

const dirs: string[] = [];

afterEach(() => {
  for (const dir of dirs.splice(0)) rmSync(dir, { recursive: true, force: true });
});

async function loadSource(source: unknown): Promise<Token[] | unknown> {
  const dir = mkdtempSync(join(tmpdir(), 'tokens-'));
  dirs.push(dir);
  writeFileSync(join(dir, 'theme.tokens.json'), JSON.stringify(source));
  return loadTokens(dir).catch((error: unknown) => error);
}

const light = (value: unknown): unknown => ({ anima: { light: value } });
const NORD = { $type: 'color', nord: { '0': { $value: '#2E3440' }, '5': { $value: '#E5E9F0' } } };

describe('light values in the token source (TOK-011)', () => {
  test('UI-005: a light alias is loaded as written, so the light block names the primitive', async () => {
    const tokens = await loadSource({ color: { ...NORD, 'surface-base': { $value: '{color.nord.0}', $extensions: light('{color.nord.5}') } } });
    expect(body(renderCss(tokens as Token[], NO_PALETTES), ':root[data-theme="light"]')).toEqual(['--surface-base: var(--nord-5);']);
  });

  test('TOK-011: a light value on a non-colour token is a TokenSourceError naming the token', async () => {
    const result = await loadSource({
      space: { $type: 'dimension', '4': { $value: '1rem', $extensions: light('2rem') } },
    });
    expect(result).toBeInstanceOf(TokenSourceError);
    expect((result as TokenSourceError).tokenPath).toBe('space.4');
  });

  test('TOK-011: a light value on a primitive colour is refused, since a theme changes only semantic colours', async () => {
    const result = await loadSource({ color: { ...NORD, nord: { ...NORD.nord, '5': { $value: '#E5E9F0', $extensions: light('#FFFFFF') } } } });
    expect(result).toBeInstanceOf(TokenSourceError);
    expect((result as TokenSourceError).tokenPath).toBe('color.nord.5');
  });

  test('UI-005: a light colour may be a mix; it renders as color-mix in both light blocks (CR-10)', async () => {
    const mix = { color: '{color.nord.0}', percent: 12, over: '{color.nord.5}' };
    const tokens = await loadSource({ color: { ...NORD, 'surface-hover': { $value: '{color.nord.0}', $extensions: light(mix) } } });
    const expected = ['--surface-hover: color-mix(in srgb, var(--nord-0) 12%, var(--nord-5));'];
    const css = renderCss(tokens as Token[], NO_PALETTES);
    expect(body(css, ':root[data-theme="light"]')).toEqual(expected);
    expect(body(css, '  :root[data-theme="system"]')).toEqual(expected);
    expect(body(css, ':root')).toContain('--surface-hover: var(--nord-0);');
  });

  test('TOK-011: a light colour mix without an over colour is refused, naming the token', async () => {
    const result = await loadSource({
      color: { ...NORD, 'surface-hover': { $value: '{color.nord.0}', $extensions: light({ color: '{color.nord.0}', percent: 12 }) } },
    });
    expect(result).toBeInstanceOf(TokenSourceError);
    expect((result as TokenSourceError).tokenPath).toBe('color.surface-hover');
  });

  test('TOK-011: a light colour that is neither a colour nor a mix is refused', async () => {
    const result = await loadSource({ color: { ...NORD, 'surface-hover': { $value: '{color.nord.0}', $extensions: light(12) } } });
    expect(result).toBeInstanceOf(TokenSourceError);
  });

  test('TOK-011: a light tint given as a plain colour is refused, since a tint is always a mix', async () => {
    const tint = { $type: 'tint', $value: { color: '{color.nord.0}', percent: 12, over: 'transparent' } };
    const result = await loadSource({ color: { ...NORD, 'accent-tint': { ...tint, $extensions: light('{color.nord.5}') } } });
    expect(result).toBeInstanceOf(TokenSourceError);
    expect((result as TokenSourceError).tokenPath).toBe('color.accent-tint');
  });

  test('TOK-011: a light tint without a numeric percent is refused', async () => {
    const tint = { $type: 'tint', $value: { color: '{color.nord.0}', percent: 12, over: 'transparent' } };
    const result = await loadSource({
      color: { ...NORD, 'accent-tint': { ...tint, $extensions: light({ color: '{color.nord.5}', percent: 'a lot', over: 'transparent' }) } },
    });
    expect(result).toBeInstanceOf(TokenSourceError);
  });
});
