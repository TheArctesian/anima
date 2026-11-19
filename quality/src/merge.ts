/** The rules of a merge of a branch into `dev` (R-198, PLN-020, PLN-021). Pure. */

/** The files that a program writes from the documents. A merge never joins two versions of them: it writes them again. */
export const GENERATED = ["docs/plan/register.md", "docs/operator-queue.md"] as const;

/**
 * The two rules that the operator can switch (`quality/merge-rule.json`):
 * `fullGates` says where the full list of the gates must pass before a merge: on the Mac of the
 * session (`mac`), or in the workflow `gates` on GitHub (`github`, R-NEW-gaterule).
 * `mergeWithoutCheck` says if the workflow `merge-to-dev` merges a branch whose gates passed as
 * merged with `dev`, with no person and no session (R-NEW-automerge).
 */
export interface MergeRule {
  fullGates: "mac" | "github";
  mergeWithoutCheck: boolean;
}

/** The rule in `text`, or the reason that it is not a rule. An unknown value is never read as "on". */
export function parseMergeRule(text: string): MergeRule | string {
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch {
    return "the rule file is not JSON";
  }
  const rule = value as Partial<MergeRule> | null;
  if (rule === null || typeof rule !== "object") return "the rule file is not an object";
  if (rule.fullGates !== "mac" && rule.fullGates !== "github") return "fullGates must be \"mac\" or \"github\"";
  if (typeof rule.mergeWithoutCheck !== "boolean") return "mergeWithoutCheck must be true or false";
  if (rule.mergeWithoutCheck && rule.fullGates !== "github") return "mergeWithoutCheck needs fullGates \"github\"";
  return { fullGates: rule.fullGates, mergeWithoutCheck: rule.mergeWithoutCheck };
}

/** The paths with a conflict that a person or a session must solve: each one that is not a generated file. */
export function conflictsForAHand(conflicted: string[]): string[] {
  return conflicted.filter((path) => !(GENERATED as readonly string[]).includes(path));
}

/** The message of the merge commit: the branch, and each number that the merge assigned. */
export function mergeMessage(branch: string, into: string, numbers: [string, string][]): string {
  const subject = `Merge ${branch} into ${into}`;
  if (numbers.length === 0) return `${subject}\n`;
  const lines = numbers.map(([placeholder, number]) => `${placeholder} is ${number}`);
  return `${subject}\n\nNumbers assigned at this merge (PLN-021):\n${lines.join("\n")}\n`;
}

/** One check run of a commit, as the API of GitHub gives it. */
export interface CheckRun {
  name: string;
  status: string;
  conclusion: string | null;
}

/** The last job of the workflow `gates`: it passes when each job of the gates passed. */
const LAST_JOB = "passed";

/** Why the gates on GitHub do not permit a merge of the commit; undefined when the job `passed` of the workflow `gates` passed. */
export function gatesRefusal(runs: CheckRun[]): string | undefined {
  const last = runs.filter((run) => run.name === LAST_JOB || run.name.endsWith(`/ ${LAST_JOB}`));
  if (last.length === 0) return "the commit has no run of the workflow `gates` (job `passed`)";
  const bad = last.find((run) => run.status !== "completed" || run.conclusion !== "success");
  return bad === undefined ? undefined : `the job \`passed\` of the gates is ${bad.status}, ${bad.conclusion ?? "no conclusion"}`;
}
