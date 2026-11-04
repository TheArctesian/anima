// Entry: `bun run graphql` writes the typed documents; `bun codegen.ts --check` fails if they are not current (API-008).
import { mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { GENERATED_FOLDER, generateTypes, staleTypes } from './src/lib/hub/generation.ts';

const clientRoot = import.meta.dirname;
const built = await generateTypes(clientRoot);

if (process.argv.includes('--check')) {
  const stale = staleTypes(built, clientRoot);
  for (const { path, reason } of stale) console.error(`${path}: ${reason}; run \`bun run --cwd apps/client graphql\` (API-008)`);
  process.exit(stale.length > 0 ? 1 : 0);
}

rmSync(join(clientRoot, GENERATED_FOLDER), { recursive: true, force: true });
for (const [path, text] of built) {
  const file = join(clientRoot, path);
  mkdirSync(dirname(file), { recursive: true });
  writeFileSync(file, text);
  console.log(`wrote ${path}`);
}
