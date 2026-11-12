// Usage: bun deploy/headscale/bin/redact.ts < captured.json > fixture.json
// Replaces every node key and pre-auth key (vpn-build.md rule 0.6); exits 1 if a secret remains.
import { redact, RedactError } from '../src/redact.ts';

try {
  process.stdout.write(redact(await Bun.stdin.text()));
} catch (error) {
  process.stderr.write(`redact: ${error instanceof RedactError ? error.message : 'not a JSON file'}\n`);
  process.exit(1);
}
