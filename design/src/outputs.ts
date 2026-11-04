import { join } from 'node:path';
import { renderCss } from './css.ts';
import { loadTokens } from './load.ts';
import { renderReference } from './reference.ts';
import { renderTypescript } from './typescript.ts';

/** Repository-relative path of each generated file, and its text. */
export type Outputs = Readonly<Record<string, string>>;

export const OUTPUT_PATHS = {
  css: 'apps/client/src/styles/tokens.css',
  typescript: 'apps/client/src/lib/motion/tokens.ts',
  reference: 'design/tokens/reference.md',
} as const;

/** Build every output in memory from the token source (TOK-002). */
export async function buildOutputs(repositoryRoot: string): Promise<Outputs> {
  const tokens = await loadTokens(join(repositoryRoot, 'design', 'tokens'));
  return {
    [OUTPUT_PATHS.css]: renderCss(tokens),
    [OUTPUT_PATHS.typescript]: renderTypescript(tokens),
    [OUTPUT_PATHS.reference]: renderReference(tokens),
  };
}
