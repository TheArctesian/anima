import type { Violation } from "../report.ts";
import type { Document } from "./documents.ts";

/** The one log of the decisions of the operator and of the programme (PLN-021). */
export const RULINGS = "docs/rulings.md";
const RULING = /^- \*\*((?:R|CR)-\d+)\b/;

/** PLN-021: a number of the log names one ruling. */
export function repeatedRulings(documents: Document[]): Violation[] {
  const log = documents.find((document) => document.path === RULINGS);
  const seen = new Set<string>();
  const violations: Violation[] = [];
  for (const line of log?.text.split("\n") ?? []) {
    const number = RULING.exec(line)?.[1];
    if (number === undefined) continue;
    if (seen.has(number)) violations.push({ path: RULINGS, signal: "ruling-number-used-twice", detail: number });
    seen.add(number);
  }
  return violations;
}
