/**
 * Phase 3 acceptance, criterion 24 (CON-002, CQ-010, P3-002): K1 holds no `@pending`
 * and the quality baseline is empty. An on-demand check, not a gate.
 *
 *   bun quality/acceptance/phase-3.ts [repo-root]
 *
 * Exit 0: clean. Exit 1: findings (`file:line: message`). Exit 2: an input is unreadable.
 */
import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";

const SCHEMA_PATH = "contracts/schema.graphql";
const BASELINE_PATH = "quality/baseline.json";

export interface Finding {
  path: string;
  line: number;
  message: string;
}

export class AcceptanceInputError extends Error {
  constructor(path: string, reason: string) {
    super(`cannot use ${path}: ${reason}`);
    this.name = "AcceptanceInputError";
  }
}

const PENDING_USE = /@pending(?![A-Za-z0-9_])/;
const PENDING_DECLARATION = /directive\s+@pending(?![A-Za-z0-9_])/;

/** `text` with string contents and `#` comments replaced by spaces; newlines stay, so line numbers hold. */
function blankStringsAndComments(text: string): string {
  const blank = (span: string): string => span.replace(/[^\n]/g, " ");
  return text.replace(/"""[\s\S]*?(?:"""|$)|"(?:\\.|[^"\\\n])*"?|#[^\n]*/g, blank);
}

/** Every `@pending` in K1 text except its declaration, comments and string contents. */
export function pendingUses(schema: string): Finding[] {
  const code = blankStringsAndComments(schema).replace(PENDING_DECLARATION, (span) => " ".repeat(span.length));
  const findings: Finding[] = [];
  code.split("\n").forEach((line, index) => {
    if (PENDING_USE.test(line)) {
      findings.push({ path: SCHEMA_PATH, line: index + 1, message: "@pending is used; remove it (P3-002)" });
    }
  });
  return findings;
}

/** One finding per key of the baseline; the baseline must be `{}` (CQ-010). */
export function baselineEntries(baselineText: string): Finding[] {
  let parsed: unknown;
  try {
    parsed = JSON.parse(baselineText);
  } catch (error) {
    throw new AcceptanceInputError(BASELINE_PATH, `not valid JSON (${String(error)})`);
  }
  if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
    throw new AcceptanceInputError(BASELINE_PATH, "not a JSON object");
  }
  return Object.keys(parsed).map((key) => ({
    path: BASELINE_PATH,
    line: 1,
    message: `baseline entry remains: ${key}`,
  }));
}

function readInput(root: string, path: string): string {
  try {
    return readFileSync(join(root, path), "utf8");
  } catch (error) {
    throw new AcceptanceInputError(path, String(error));
  }
}

/** Run both checks against the repository at `root`. */
export function checkPhase3(root: string): Finding[] {
  return [
    ...pendingUses(readInput(root, SCHEMA_PATH)),
    ...baselineEntries(readInput(root, BASELINE_PATH)),
  ];
}

function main(root: string): number {
  try {
    const findings = checkPhase3(root);
    for (const finding of findings) {
      console.error(`${finding.path}:${finding.line}: ${finding.message}`);
    }
    if (findings.length > 0) return 1;
    console.log("acceptance phase-3: ok");
    return 0;
  } catch (error) {
    if (error instanceof AcceptanceInputError) {
      console.error(error.message);
      return 2;
    }
    throw error;
  }
}

if (import.meta.main) {
  process.exit(main(resolve(process.argv[2] ?? join(import.meta.dir, "..", ".."))));
}
