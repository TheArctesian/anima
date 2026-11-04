// Entry: `bun run tokens` writes the outputs; `bun run tokens:check` fails if any is not current (TOK-002).
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { buildOutputs } from '../src/outputs.ts';
import { staleOutputs } from '../src/staleness.ts';

const repositoryRoot = resolve(import.meta.dir, '..', '..');

if (process.argv.includes('--check')) {
  const stale = await staleOutputs(repositoryRoot);
  for (const { path, reason } of stale) console.error(`${path}: ${reason}; run \`bun run --cwd design tokens\` (TOK-002)`);
  process.exit(stale.length > 0 ? 1 : 0);
}

for (const [path, text] of Object.entries(await buildOutputs(repositoryRoot))) {
  const file = join(repositoryRoot, path);
  mkdirSync(dirname(file), { recursive: true });
  writeFileSync(file, text);
  console.log(`wrote ${path}`);
}
