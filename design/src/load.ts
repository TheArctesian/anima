import { readdirSync } from 'node:fs';
import { join } from 'node:path';
import StyleDictionary from 'style-dictionary';
import { tokenName, type Token } from './model.ts';

/** Source files in a fixed order, so every output is deterministic. */
export function tokenSources(tokensDir: string): string[] {
  return readdirSync(tokensDir)
    .filter((file) => file.endsWith('.tokens.json'))
    .sort()
    .map((file) => join(tokensDir, file));
}

function extensionReduced(extensions: unknown): unknown {
  if (typeof extensions !== 'object' || extensions === null) return undefined;
  const anima = (extensions as Record<string, unknown>)['anima'];
  if (typeof anima !== 'object' || anima === null) return undefined;
  return (anima as Record<string, unknown>)['reduced'];
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
  return allTokens.map((token) => ({
    name: tokenName(token.path),
    group: token.path.slice(0, -1).join('.'),
    type: String(token.$type ?? ''),
    value: token.$value,
    source: token.original.$value,
    description: String(token.$description ?? ''),
    reduced: extensionReduced(token.$extensions),
  }));
}
