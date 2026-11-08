import { readdirSync } from 'node:fs';
import { join } from 'node:path';
import StyleDictionary, { type TransformedToken } from 'style-dictionary';
import { bezierOf, isSemanticColour, springOf, tokenName, TokenSourceError, type ModeValue, type Token } from './model.ts';

/** Source files in a fixed order, so every output is deterministic. */
export function tokenSources(tokensDir: string): string[] {
  return readdirSync(tokensDir)
    .filter((file) => file.endsWith('.tokens.json'))
    .sort()
    .map((file) => join(tokensDir, file));
}

/** A mode value under `$extensions.anima` (`reduced`, `light`), or undefined. */
function animaExtension(extensions: unknown, mode: string): unknown {
  if (typeof extensions !== 'object' || extensions === null) return undefined;
  const anima = (extensions as Record<string, unknown>)['anima'];
  if (typeof anima !== 'object' || anima === null) return undefined;
  return (anima as Record<string, unknown>)[mode];
}

/** The light value as written and as resolved: Style Dictionary resolves aliases inside extensions too. */
function lightOf(token: TransformedToken): ModeValue | undefined {
  const source = animaExtension(token.original.$extensions, 'light');
  return source === undefined ? undefined : { source, value: animaExtension(token.$extensions, 'light') };
}

/** Read the DTCG files with Style Dictionary, which resolves every alias (D-2). */
export async function loadTokens(tokensDir: string): Promise<Token[]> {
  const dictionary = new StyleDictionary({
    source: tokenSources(tokensDir),
    usesDtcg: true,
    log: { verbosity: 'silent', warnings: 'error' },
    platforms: { interface: { transforms: [] } },
  });
  const { allTokens } = await dictionary.getPlatformTokens('interface');
  const tokens = allTokens.map((token): Token => ({
    name: tokenName(token.path),
    path: token.path.join('.'),
    group: token.path.slice(0, -1).join('.'),
    type: String(token.$type ?? ''),
    value: token.$value,
    source: token.original.$value,
    description: String(token.$description ?? ''),
    reduced: animaExtension(token.$extensions, 'reduced'),
    light: lightOf(token),
  }));
  for (const token of tokens) validated(token);
  return tokens;
}

/** Shapes the renderers rely on are checked here once, so no NaN reaches an output. */
function validated(token: Token): void {
  if (token.type === 'spring') springOf(token);
  if (token.type === 'cubicBezier') bezierOf(token);
  if (token.light !== undefined) lightChecked(token, token.light.value);
}

function isTintShape(value: unknown): boolean {
  if (typeof value !== 'object' || value === null) return false;
  const { color, percent, over } = value as Record<string, unknown>;
  return typeof color === 'string' && typeof over === 'string' && typeof percent === 'number' && Number.isFinite(percent);
}

/**
 * A theme changes only semantic colours (TOK-011). A light tint is a mix; a light colour is a colour or a mix
 * (`surface-hover`, CR-10).
 */
function lightChecked(token: Token, value: unknown): void {
  if (!isSemanticColour(token)) {
    throw new TokenSourceError(token.path, 'a light value is allowed only on a semantic colour (TOK-011)');
  }
  const shaped = isTintShape(value) || (token.type === 'color' && typeof value === 'string');
  if (!shaped) throw new TokenSourceError(token.path, `a light ${token.type} value has the wrong shape`);
}
