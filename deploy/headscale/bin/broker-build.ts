// Usage: bun deploy/headscale/bin/broker-build.ts [arm64]
// Compiles the broker into one Linux binary, deploy/headscale/broker/dist/anima-broker (BD-7),
// and prints its SHA-256 for `harden.sh --sha256=<hash> broker` (L5).
// The first cross-compile downloads the Bun runtime for the target (network).
import { join } from 'node:path';

const [arch, ...rest] = process.argv.slice(2);
if (rest.length > 0 || (arch !== undefined && arch !== 'arm64')) {
  process.stderr.write('usage: bun deploy/headscale/bin/broker-build.ts [arm64]\n');
  process.exit(64);
}

const broker = join(import.meta.dir, '..', 'broker');
const target = arch === 'arm64' ? 'bun-linux-arm64' : 'bun-linux-x64-baseline';
const outfile = join(broker, 'dist', 'anima-broker');
const build = Bun.spawnSync(['bun', 'build', join(broker, 'main.ts'), '--compile', `--target=${target}`, `--outfile=${outfile}`], { stdout: 'inherit', stderr: 'inherit' });
if (build.exitCode !== 0) process.exit(build.exitCode);
const sha256 = new Bun.CryptoHasher('sha256').update(await Bun.file(outfile).arrayBuffer()).digest('hex');
process.stdout.write(`${outfile} (${target})\nsha256 ${sha256}\n`);
