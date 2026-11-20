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
  /**
   * If a train whose gates passed still lands when `dev` moved under it only in documents. Off
   * when the file does not say `true`: then the gates run again for the train on the new `dev`.
   */
  carryDocuments: boolean;
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
  return { fullGates: rule.fullGates, mergeWithoutCheck: rule.mergeWithoutCheck, carryDocuments: rule.carryDocuments === true };
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

/** The table of the decisions of the work plan: each session adds rows to it, so it merges by union (`.gitattributes`). */
export const DECISIONS = "docs/plan/decisions.md";

/**
 * The identifiers that are the first cell of more than one row of a table in `text`. A union
 * merge keeps the two versions of a row that two branches changed; this finds them.
 */
export function rowsTwice(text: string): string[] {
  const seen = new Set<string>();
  const twice = new Set<string>();
  for (const line of text.split("\n")) {
    const id = /^\| ([A-Z][A-Za-z0-9-]*-[A-Za-z0-9-]+) \|/.exec(line)?.[1];
    if (id === undefined) continue;
    if (seen.has(id)) twice.add(id);
    seen.add(id);
  }
  return [...twice];
}


/** The list of the security findings and the table of owners: each branch adds entries, so they merge by union too. */
export const FINDINGS = "docs/security-findings.md";
export const OWNERS = "docs/plan/ownership.md";

/**
 * The keys that more than one row of the same table has. A table ends at the first line that is
 * not a row, so an identifier can be in two tables of one document. `key` gives the key of a
 * row from its cells, or undefined for a row that has none.
 */
export function twiceInOneTable(text: string, key: (cells: string[]) => string | undefined): string[] {
  let seen = new Set<string>();
  const twice = new Set<string>();
  for (const line of text.split("\n")) {
    if (!line.startsWith("|")) {
      seen = new Set();
      continue;
    }
    const id = key(line.split("|").slice(1, -1).map((cell) => cell.trim()));
    if (id === undefined) continue;
    if (seen.has(id)) twice.add(id);
    seen.add(id);
  }
  return [...twice];
}

/** The headings of the form `### SF-nnn:` that are in `text` more than one time. */
export function headingsTwice(text: string): string[] {
  const all = [...text.matchAll(/^### ([A-Z]+-[A-Za-z0-9-]+):/gm)].map((match) => match[1] ?? "");
  return [...new Set(all.filter((id, at) => all.indexOf(id) !== at))];
}

/** The key of a row of the findings list: its identifier. */
export function findingKey(cells: string[]): string | undefined {
  return /^SF-[A-Za-z0-9-]+$/.exec(cells[0] ?? "")?.[0];
}

/** The key of a row of the owners table: its identifiers, when the cell names a number. */
export function ownerKey(cells: string[]): string | undefined {
  const identifiers = cells.at(-1) ?? "";
  return /\d/.test(identifiers) ? identifiers : undefined;
}
