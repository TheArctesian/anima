import { escapedAttempts } from './escapes';
import { startedProxies } from './shared-proxy';

// After the last test of every project: the deny-all proxy of each engine must hold nothing the harness does not explain.
// A throw here fails the run, so a test browser that reaches for a real host cannot pass unseen (WP-44 ruling S3).

export default async function checkNothingEscaped(): Promise<void> {
  const escaped: string[] = [];
  for (const [engine, proxy] of startedProxies()) {
    const targets = escapedAttempts(engine, await proxy.attempts()).map((attempt) => `${attempt.via} ${attempt.target}`);
    if (targets.length > 0) escaped.push(`${engine}: ${[...new Set(targets)].join(', ')}`);
    await proxy.close();
  }
  if (escaped.length > 0) throw new Error(`a test browser reached past the harness; the deny-all proxy refused it:\n${escaped.join('\n')}`);
}
