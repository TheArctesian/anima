import { describe, expect, test } from 'bun:test';
import { resolve } from 'node:path';
import { renderCss } from '../src/css.ts';
import { loadTokens } from '../src/load.ts';
import type { Token } from '../src/model.ts';
import { palettes } from './support.ts';

function token(name: string, type: string, value: unknown, reduced?: unknown): Token {
  return { name, path: `motion.${name}`, group: 'motion', type, value, source: value, description: '', reduced, light: undefined, palette: undefined };
}

/** A synthetic motion set: press goes to 0 under reduced motion, base stays, the spring jumps, the threshold never changes. */
const MOTION: Token[] = [
  token('duration-press', 'duration', '80ms', '0ms'),
  token('duration-base', 'duration', '240ms'),
  token('spring-test', 'spring', { dampingRatio: 1, responseS: 0.3 }),
  token('threshold-drag', 'dimension', '12px'),
  { ...token('surface-base', 'color', '#101010', undefined), group: 'color', light: { source: '#F0F0F0', value: '#F0F0F0' } },
];
const STILL: Token[] = MOTION.filter((each) => each.type !== 'spring').map((each) => ({ ...each, reduced: undefined }));

/** The declarations inside the first block that opens with `opening`. */
function body(css: string, opening: string): string[] {
  const start = css.indexOf(`${opening} {`);
  if (start < 0) return [];
  const text = css.slice(start + opening.length + 2, css.indexOf('}', start));
  return text.split('\n').map((line) => line.trim()).filter((line) => line !== '');
}

/** The value a block gives one custom property. */
function valueIn(lines: readonly string[], property: string): string | undefined {
  const line = lines.find((each) => each.startsWith(`${property}: `));
  return line?.slice(property.length + 2, -1);
}

describe('Reduce motion in Settings (MOT-043, D-28, F-7)', () => {
  const css = renderCss(MOTION, []);
  const reducedByOs = body(css, '@media (prefers-reduced-motion: reduce) {\n  :root');

  test('MOT-040: System leaves the media query in charge: its block is still plain :root with the reduced values', () => {
    expect(css).toContain('@media (prefers-reduced-motion: reduce) {\n  :root {\n');
    expect(reducedByOs).toEqual(['--duration-press: 0ms;', '--spring-test: 0ms linear;']);
  });

  test('MOT-043: On (data-motion="reduce") takes the reduced values with no media query around it', () => {
    expect(css).toContain('\n:root[data-motion="reduce"] {\n');
    expect(body(css, ':root[data-motion="reduce"]')).toEqual(reducedByOs);
  });

  test('MOT-043: Off (data-motion="full") puts back the full values of exactly the tokens a reducing OS changes', () => {
    expect(css).toContain('@media (prefers-reduced-motion: reduce) {\n  :root[data-motion="full"] {\n');
    const full = body(css, '  :root[data-motion="full"]');
    const root = body(css, ':root');
    expect(full.map((line) => line.split(':')[0])).toEqual(reducedByOs.map((line) => line.split(':')[0]));
    for (const property of ['--duration-press', '--spring-test']) expect(valueIn(full, property)).toBe(valueIn(root, property));
  });

  test('MOT-021: neither choice touches a threshold or a token reduced motion keeps', () => {
    for (const opening of [':root[data-motion="reduce"]', '  :root[data-motion="full"]']) {
      const names = body(css, opening).map((line) => line.split(':')[0]);
      expect(names).not.toContain('--threshold-drag');
      expect(names).not.toContain('--duration-base');
    }
  });

  test('MOT-043: the choice blocks name no colour, so they never compete with a theme block (TOK-011)', () => {
    const choice = [...body(css, ':root[data-motion="reduce"]'), ...body(css, '  :root[data-motion="full"]')];
    expect(choice.some((line) => line.startsWith('--surface-base:'))).toBe(false);
  });

  test('MOT-043: the choice blocks come right after the media block, before the transparency and contrast blocks', () => {
    const media = css.indexOf('@media (prefers-reduced-motion: reduce) {\n  :root {');
    const reduce = css.indexOf(':root[data-motion="reduce"]');
    const full = css.indexOf(':root[data-motion="full"]');
    const transparency = css.indexOf('@media (prefers-reduced-transparency: reduce)');
    expect(media).toBeLessThan(reduce);
    expect(reduce).toBeLessThan(full);
    expect(full).toBeLessThan(transparency);
  });

  test('MOT-043: without a token that changes under reduced motion there is no choice block', () => {
    expect(renderCss(STILL, [])).not.toContain('data-motion');
  });
});

describe('golden: the output without data-motion is unchanged (R-085, MOT-040)', () => {
  test('MOT-040: cutting the two choice blocks out of tokens.css leaves the same blocks, line for line, as before T4', async () => {
    const tokens = await loadTokens(resolve(import.meta.dir, '..', 'tokens'));
    const css = renderCss(tokens, palettes);
    const start = css.indexOf('/* Reduce motion in Settings (MOT-043)');
    const end = css.indexOf('/* Reduced transparency (5.6)');
    const added = css.slice(start, end);
    const rest = css.slice(0, start) + css.slice(end);
    // Every line the choice adds names data-motion or sits inside one of its two blocks.
    expect(added.split('\n').filter((line) => line.includes('data-motion'))).toHaveLength(2);
    expect(rest).not.toContain('data-motion');
    // What is left is :root, then the two palettes (Graphite on :root, Nord by attribute),
    // then the reduced-motion media block and the two preference media blocks, in order (UI-023, MAT-006).
    const openings = rest.split('\n').filter((line) => line.endsWith('{'));
    const preference = ':root, :root[data-theme], :root[data-palette], :root[data-palette][data-theme] {';
    expect(openings).toEqual([
      ':root {',
      ':root, :root[data-palette="graphite"] {',
      ':root[data-theme="light"] {',
      '@media (prefers-color-scheme: light) {',
      '  :root[data-theme="system"] {',
      ':root[data-palette="nord"] {',
      ':root[data-palette="nord"][data-theme="light"] {',
      '@media (prefers-color-scheme: light) {',
      '  :root[data-palette="nord"][data-theme="system"] {',
      '@media (prefers-reduced-motion: reduce) {',
      '  :root {',
      '@media (prefers-reduced-transparency: reduce) {',
      `  ${preference}`,
      '@media (prefers-contrast: more) {',
      `  ${preference}`,
    ]);
  });
});
