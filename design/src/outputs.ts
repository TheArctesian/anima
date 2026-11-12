import { join } from 'node:path';
import { renderCss } from './css.ts';
import { loadPalettes, loadTokens } from './load.ts';
import { isTerminal } from './model.ts';
import { renderReference } from './reference.ts';
import { renderRust } from './rust.ts';
import { renderTypescript } from './typescript.ts';

/** Repository-relative path of each generated file, and its text. */
export type Outputs = Readonly<Record<string, string>>;

export const OUTPUT_PATHS = {
  css: 'apps/client/src/styles/tokens.css',
  typescript: 'apps/client/src/lib/motion/tokens.ts',
  rust: 'crates/theme/src/generated.rs',
  reference: 'design/tokens/reference.md',
} as const;

/** Build every output in memory from the token source (TOK-002). */
export async function buildOutputs(repositoryRoot: string): Promise<Outputs> {
  const tokensDir = join(repositoryRoot, 'design', 'tokens');
  const all = await loadTokens(tokensDir);
  const interfaceTokens = all.filter((token) => !isTerminal(token));
  const palettes = loadPalettes(tokensDir);
  return {
    [OUTPUT_PATHS.css]: renderCss(interfaceTokens, palettes),
    [OUTPUT_PATHS.typescript]: renderTypescript(interfaceTokens),
    [OUTPUT_PATHS.rust]: renderRust(all),
    [OUTPUT_PATHS.reference]: renderReference(interfaceTokens),
  };
}
