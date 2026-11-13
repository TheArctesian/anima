// Shared helpers for the palette tests: resolve a role for one palette and one appearance, following its source aliases.
import { resolve } from 'node:path';
import { contrastRatio, mixColour, type HexColour } from '../src/contrast.ts';
import { loadPalettes, loadTokens } from '../src/load.ts';
import type { Palette, Token } from '../src/model.ts';

export const TOKENS_DIR = resolve(import.meta.dir, '..', 'tokens');
export const tokens: Token[] = await loadTokens(TOKENS_DIR);
export const palettes: Palette[] = loadPalettes(TOKENS_DIR);

export type Appearance = 'dark' | 'light';
export const APPEARANCES: readonly Appearance[] = ['dark', 'light'];

/** Every semantic role of Section 5.1.2 (leaf names; each palette fills each one). */
export const ROLE_NAMES: readonly string[] = [
  'surface-sunken', 'surface-base', 'surface-raised', 'surface-overlay', 'surface-hover', 'surface-selected',
  'border-subtle', 'border-strong',
  'text', 'text-secondary', 'text-tertiary',
  'accent', 'accent-hover', 'accent-pressed', 'on-accent', 'accent-tint',
  ...['up', 'down', 'degraded', 'warning', 'maintenance', 'unknown'].map((name) => `status-${name}`),
  ...['up', 'down', 'degraded', 'warning', 'maintenance', 'unknown'].map((name) => `status-${name}-text`),
  ...['up', 'down', 'degraded', 'warning', 'maintenance', 'unknown'].map((name) => `status-${name}-tint`),
  'danger', 'on-danger', 'scrim', 'focus-ring', 'shadow',
  'chart-grid', 'chart-axis', 'chart-threshold',
  ...[1, 2, 3, 4, 5, 6].map((n) => `chart-series-${String(n)}`),
];

/** The surface tiers a text colour must reach 4.5:1 on (UI-002). */
export const SURFACES: readonly string[] = ['surface-sunken', 'surface-base', 'surface-raised', 'surface-overlay', 'surface-hover'];
export const STATUS_SHAPES: readonly string[] = ['status-up', 'status-down', 'status-degraded', 'status-warning', 'status-maintenance', 'status-unknown'];

/** The role token of one palette. */
export function roleToken(palette: string, role: string): Token {
  const token = tokens.find((candidate) => candidate.palette === palette && candidate.name === role);
  if (!token) throw new Error(`no ${palette} role ${role}`);
  return token;
}

/** The hex value of a primitive, by its source path such as `color.nord.0-sunken`. */
function primitiveHex(path: string): HexColour {
  const token = tokens.find((candidate) => candidate.path === path);
  if (typeof token?.value !== 'string') throw new Error(`no primitive ${path}`);
  return token.value as HexColour;
}

/** A source value as a hex: an alias to a role (same palette or any), an alias to a primitive, a plain colour or a mix. */
export function resolveSource(source: unknown, appearance: Appearance): HexColour {
  if (typeof source === 'string') {
    const alias = /^\{(.+)\}$/.exec(source)?.[1];
    if (alias === undefined) return source as HexColour;
    const parts = alias.split('.');
    if (parts[1] === 'role') return hexOf(parts[2] ?? '', parts.slice(3).join('-'), appearance);
    return primitiveHex(alias);
  }
  const mix = source as { color: unknown; percent: number; over: unknown };
  return mixColour(resolveSource(mix.over, appearance), resolveSource(mix.color, appearance), mix.percent / 100);
}

/** The hex a palette's role shows in one appearance, as CSS shows it (a mix is computed as color-mix in sRGB does). */
export function hexOf(palette: string, role: string, appearance: Appearance): HexColour {
  const token = roleToken(palette, role);
  const source = appearance === 'light' ? (token.light?.source ?? token.source) : token.source;
  return resolveSource(source, appearance);
}

/** The WCAG ratio between two roles of one palette and appearance. */
export function ratioOf(palette: string, foreground: string, background: string, appearance: Appearance): number {
  return contrastRatio(hexOf(palette, foreground, appearance), hexOf(palette, background, appearance));
}
