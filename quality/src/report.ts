/** One broken rule at one path. `detail` reads like `301 > 300`. */
export interface Violation {
  path: string;
  signal: string;
  detail: string;
}

export function formatViolation(violation: Violation): string {
  return `${violation.path}: ${violation.signal} ${violation.detail}`;
}

/** Print violations and return the process exit code (0 when there are none). */
export function report(check: string, violations: Violation[]): number {
  for (const violation of violations) {
    console.error(formatViolation(violation));
  }
  if (violations.length > 0) {
    console.error(`${check}: ${violations.length} violation(s)`);
    return 1;
  }
  console.log(`${check}: ok`);
  return 0;
}
