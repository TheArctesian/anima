import { doorDots } from '../../apps/client/src/routes/sign-in/doors/doors.ts';
import { declarations } from './css.ts';
import { GENERATED_NOTICE, type Palette, type Token } from './model.ts';

const INSTALLED_TOKENS = new Set([
  'surface-sunken', 'text', 'text-secondary', 'accent', 'font-ui',
  'space-4', 'space-6', 'space-8', 'space-12', 'space-16', 'text-md', 'text-2xl', 'text-display',
  'weight-light', 'leading-tight', 'leading-normal', 'tracking-tight',
]);

/** The hub's standalone page follows the default palette and the browser's appearance (TOK-001). */
export function renderInstalledTokens(tokens: readonly Token[], palettes: readonly Palette[]): string {
  const roles = palettes.find((palette) => palette.default)?.id;
  const used = tokens.filter((token) => INSTALLED_TOKENS.has(token.name) && (token.palette === undefined || token.palette === roles));
  const resolved = (token: Token, value: unknown): string[] =>
    declarations({ ...token, value, source: value }).map(([name, css]) => `  ${name}: ${css};`);
  const dark = used.flatMap((token) => resolved(token, token.value));
  const light = used.flatMap((token) => token.light === undefined ? [] : resolved(token, token.light.value));
  return [
    `/* ${GENERATED_NOTICE} */`, ':root {', ...dark, '}',
    '@media (prefers-color-scheme: light) {', '  :root {', ...light.map((line) => `  ${line}`), '  }', '}', '',
  ].join('\n');
}

/** A still frame of the sign-in doors. A fixed seed keeps the published artwork reproducible (UR-7, D13). */
export function renderDoors(): string {
  let seed = 13;
  const dots = doorDots(1800, () => {
    seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0;
    return seed / 2 ** 32;
  });
  const layers = Array.from({ length: 5 }, (_, depth) => {
    const path = dots.filter((dot) => Math.floor(dot.depth) === depth)
      .map(({ x, y }) => `M${Math.round(220 + x * 208)} ${Math.round(220 + y * 208)}h.1`).join('');
    const opacity = (0.5 + depth * 0.1).toFixed(1);
    return `  <path opacity="${opacity}" d="${path}"/>`;
  });
  return [
    '<!-- generated from the sign-in doors by design/src/github-installation.ts (UR-7, D13) -->',
    '<svg class="doors" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 440 440" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round">',
    ...layers, '</svg>', '',
  ].join('\n');
}
