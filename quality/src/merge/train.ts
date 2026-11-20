/**
 * The rules of the merge train (R-252). A train is a batch of branches whose own gates passed,
 * merged in a fixed order on the `dev` of the moment, in a temporary branch `train/<name>`. The
 * gates run one time on that commit. When they pass and `dev` did not move, `dev` moves to that
 * commit: the tree that was tested is the tree that `dev` gets. Pure.
 */

/** The start of the name of each train branch. A session never pushes a branch with this start. */
export const TRAIN = "train/";
/** The most branches in one train. A failed train of n branches costs about log2(n) more runs. */
export const MOST = 12;

/** A run of the workflow `gates`, as `gh run list` gives it. */
export interface GatesRun {
  databaseId: number;
  headBranch: string;
  headSha: string;
  event: string;
  status: string;
  conclusion: string | null;
  updatedAt: string;
}

/** A branch on origin and its tip. */
export interface Tip {
  branch: string;
  sha: string;
}

/** The refusals of earlier trains: the tip of a branch that failed alone, and when (ISO time). */
export type Refused = ReadonlyMap<string, string>;

function isWork(branch: string): boolean {
  return branch !== "dev" && branch !== "main" && branch !== "HEAD" && !branch.startsWith(TRAIN);
}

/** The time at which the gates last passed for exactly this tip, by a push of the branch. */
function passedAt(tip: Tip, runs: GatesRun[]): string | undefined {
  const green = runs.filter((run) => run.event === "push" && run.headSha === tip.sha && run.status === "completed" && run.conclusion === "success");
  return green.map((run) => run.updatedAt).sort().at(-1);
}

/**
 * The branches of the next train, in their order: each branch that is not in `dev`, whose tip
 * passed its own gates, the oldest pass first and then by name. A tip that failed alone in a
 * train stays out until its gates pass again after that failure or the branch moves.
 */
export function batch(tips: Tip[], runs: GatesRun[], refused: Refused, inDev: (sha: string) => boolean): Tip[] {
  const ready = tips.flatMap((tip) => {
    if (!isWork(tip.branch) || inDev(tip.sha)) return [];
    const at = passedAt(tip, runs);
    if (at === undefined || (refused.get(tip.sha) ?? "") >= at) return [];
    return [{ tip, at }];
  });
  ready.sort((a, b) => (a.at === b.at ? (a.tip.branch < b.tip.branch ? -1 : 1) : a.at < b.at ? -1 : 1));
  return ready.slice(0, MOST).map((each) => each.tip);
}

/** What the conductor does with a train that exists. */
export type Verdict = "start-gates" | "wait" | "land" | "split" | "build-again";

/**
 * The verdict for a train from the runs of the gates on its commit. No run: start one. A run
 * that did not end: wait. Passed: land. Failed: split. Cancelled: no verdict, so build it again.
 */
export function verdict(trainSha: string, runs: GatesRun[]): Verdict {
  const mine = runs.filter((run) => run.headSha === trainSha && run.headBranch.startsWith(TRAIN));
  if (mine.length === 0) return "start-gates";
  if (mine.some((run) => run.status !== "completed")) return "wait";
  if (mine.some((run) => run.conclusion === "success")) return "land";
  return mine.some((run) => run.conclusion === "failure" || run.conclusion === "timed_out") ? "split" : "build-again";
}

/**
 * After a train of more than one branch failed: the branches of the next train, the first half
 * in the same order. A train of one branch that failed has no half: that branch is refused.
 */
export function firstHalf<T>(members: T[]): T[] {
  return members.slice(0, Math.ceil(members.length / 2));
}

/** The name of a train branch: the commit of `dev` that it stands on, and the time, so that two trains never share a name. */
export function trainName(devSha: string, now: Date): string {
  return `${TRAIN}${devSha.slice(0, 8)}-${now.toISOString().replace(/[-:]|\.\d+Z$/g, "").toLowerCase()}`;
}

/** The branch that a merge commit of the merge script names in its subject, or undefined. */
export function mergedBranch(subject: string): string | undefined {
  return /^Merge origin\/(\S+) into dev$/.exec(subject)?.[1];
}
