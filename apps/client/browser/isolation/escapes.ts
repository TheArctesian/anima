import type { Attempt } from './deny-proxy';

// What may reach a deny-all proxy without failing the run (WP-44 ruling S3). The proxy refuses everything, so nothing
// here ever reaches a host; this says which refused attempts the harness itself explains.

/** Hosts the harness asks for on purpose, all under `.invalid` (RFC 6761), which no resolver answers. */
export const HARNESS_TARGETS: readonly string[] = [
  // Firefox sends every CSP report past `context.route`, straight to the network (csp/violations.ts).
  'csp-report.invalid:443',
  // The end-of-test marker of csp/violations.ts, when an engine tries it before the policy refuses it.
  'csp-sentinel.invalid:443',
  // The isolation spec's own attempt, which proves the proxy sits in the path of every engine.
  'isolation-probe.invalid:443',
  'isolation-probe.invalid:80',
];

/**
 * WebKit opens a TLS connection to the page's origin when a navigation starts, before `context.route` can answer it. With
 * the proxy it ends in a refusal; no other engine does it, and the hub origin is the only host it asks for.
 */
const ENGINE_TARGETS: Readonly<Record<string, readonly string[]>> = { webkit: ['hub.anima.ing:443'] };

/** The attempts of `engine`'s proxy that the harness does not explain: each one is a browser reaching for a host. */
export function escapedAttempts(engine: string, attempts: readonly Attempt[]): Attempt[] {
  const explained = [...HARNESS_TARGETS, ...(ENGINE_TARGETS[engine] ?? [])];
  return attempts.filter((attempt) => !explained.includes(attempt.target));
}
