// Entry: `bun run tokens` writes the outputs; `bun run tokens:check` fails if any is not current (TOK-002).
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { staleFiles } from '../src/currency.ts';
import { buildOutputs } from '../src/outputs.ts';

const repositoryRoot = resolve(import.meta.dir, '..', '..');
const built = await buildOutputs(repositoryRoot);

if (process.argv.includes('--check')) {
  const onDisk = Object.fromEntries(
    Object.keys(built).map((path) => {
      const file = join(repositoryRoot, path);
      return [path, existsSync(file) ? readFileSync(file, 'utf8') : undefined];
    }),
  );
  const stale = staleFiles(built, onDisk);
  for (const { path, reason } of stale) console.error(`${path}: ${reason}; run \`bun run --cwd design tokens\` (TOK-002)`);
  process.exit(stale.length > 0 ? 1 : 0);
}

for (const [path, text] of Object.entries(built)) {
  const file = join(repositoryRoot, path);
  mkdirSync(dirname(file), { recursive: true });
  writeFileSync(file, text);
  console.log(`wrote ${path}`);
}
