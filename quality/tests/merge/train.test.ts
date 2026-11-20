import { describe, expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { commitThenCheck } from "../../src/merge/commit.ts";
import { members, stack } from "../../src/merge/stack.ts";
import { batch, firstHalf, MOST, standing, trainName, verdict, type GatesRun, type Tip } from "../../src/merge/train.ts";

function gatesRun(headBranch: string, headSha: string, conclusion: string | null, updatedAt = "2026-11-20T04:00:00Z"): GatesRun {
  const event = headBranch.startsWith("train/") ? "workflow_dispatch" : "push";
  return { databaseId: 1, headBranch, headSha, event, status: conclusion === null ? "in_progress" : "completed", conclusion, updatedAt };
}

const notInDev = (): boolean => false;

describe("R-252 which branches the next train holds", () => {
  const tips: Tip[] = [
    { branch: "ui/b", sha: "b1" },
    { branch: "hub/a", sha: "a1" },
    { branch: "docs/c", sha: "c2" },
    { branch: "dev", sha: "d0" },
    { branch: "main", sha: "m0" },
    { branch: "train/d0-x", sha: "t0" },
  ];

  test("each branch whose tip passed its own gates, the oldest pass first; never dev, main or a train", () => {
    const runs = [
      gatesRun("ui/b", "b1", "success", "2026-11-20T04:10:00Z"),
      gatesRun("hub/a", "a1", "success", "2026-11-20T04:30:00Z"),
      gatesRun("dev", "d0", "success"),
      gatesRun("train/d0-x", "t0", "success"),
    ];
    expect(batch(tips, runs, new Map(), notInDev).map((tip) => tip.branch)).toEqual(["ui/b", "hub/a"]);
  });

  test("a pass of an older commit of the branch does not count, and no run that failed or still runs", () => {
    const runs = [gatesRun("docs/c", "c1", "success"), gatesRun("ui/b", "b1", "failure"), gatesRun("hub/a", "a1", null)];
    expect(batch(tips, runs, new Map(), notInDev)).toEqual([]);
  });

  test("a branch that dev already holds is not taken again", () => {
    const runs = [gatesRun("ui/b", "b1", "success"), gatesRun("hub/a", "a1", "success")];
    expect(batch(tips, runs, new Map(), (sha) => sha === "b1").map((tip) => tip.branch)).toEqual(["hub/a"]);
  });

  test("a tip that failed alone stays out until its own gates pass again after that failure", () => {
    const runs = [gatesRun("ui/b", "b1", "success", "2026-11-20T04:10:00Z")];
    expect(batch(tips, runs, new Map([["b1", "2026-11-20T05:00:00.000Z"]]), notInDev)).toEqual([]);
    const again = [...runs, gatesRun("ui/b", "b1", "success", "2026-11-20T05:40:00Z")];
    expect(batch(tips, again, new Map([["b1", "2026-11-20T05:00:00.000Z"]]), notInDev)).toHaveLength(1);
  });

  test("each branch that is not taken has a reason, and a branch in dev is not named", () => {
    const runs = [
      gatesRun("ui/b", "b1", "failure"),
      gatesRun("hub/a", "a1", null),
      gatesRun("docs/c", "c1", "success"),
      gatesRun("hosted/d", "d1", "success", "2026-11-20T04:10:00Z"),
    ];
    const all = [...tips, { branch: "hosted/d", sha: "d1" }, { branch: "new/e", sha: "e1" }, { branch: "old/f", sha: "f1" }];
    const reasons = new Map(standing(all, runs, new Map([["d1", "2026-11-20T05:00:00.000Z"]]), (sha) => sha === "f1").map((each) => [each.tip.branch, each.reason]));
    expect(reasons.get("ui/b")).toContain('ended with "failure"');
    expect(reasons.get("hub/a")).toBe("its gates still run");
    expect(reasons.get("docs/c")).toBe("its gates passed for an older commit (c1), not for its tip");
    expect(reasons.get("hosted/d")).toContain("failed alone in a train");
    expect(reasons.get("new/e")).toBe("its gates did not run for its tip");
    expect([...reasons.keys()].sort()).toEqual(["docs/c", "hosted/d", "hub/a", "new/e", "ui/b"]);
  });

  test("one train holds a limited number of branches", () => {
    const many = Array.from({ length: MOST + 5 }, (_, at) => ({ branch: `work/${String(at).padStart(2, "0")}`, sha: `s${at}` }));
    expect(batch(many, many.map((tip) => gatesRun(tip.branch, tip.sha, "success")), new Map(), notInDev)).toHaveLength(MOST);
  });
});

describe("R-252 what happens to a train", () => {
  test("it lands only when the gates passed for its own commit", () => {
    expect(verdict("t1", [])).toBe("start-gates");
    expect(verdict("t1", [gatesRun("train/x", "t0", "success")])).toBe("start-gates");
    expect(verdict("t1", [gatesRun("train/x", "t1", null)])).toBe("wait");
    expect(verdict("t1", [gatesRun("train/x", "t1", "success")])).toBe("land");
    expect(verdict("t1", [gatesRun("ui/b", "t1", "success")])).toBe("start-gates");
  });

  test("a failure splits it, and a cancelled run is no verdict", () => {
    expect(verdict("t1", [gatesRun("train/x", "t1", "failure")])).toBe("split");
    expect(verdict("t1", [gatesRun("train/x", "t1", "cancelled")])).toBe("build-again");
    expect(firstHalf(["a", "b", "c", "d", "e"])).toEqual(["a", "b", "c"]);
    expect(firstHalf(["a", "b"])).toEqual(["a"]);
  });

  test("two trains on one commit of dev have different names", () => {
    const sha = "0123456789abcdef";
    expect(trainName(sha, new Date("2026-11-20T04:30:58Z"))).toBe("train/01234567-20261120t043058");
    expect(trainName(sha, new Date("2026-11-20T04:30:59Z"))).not.toBe(trainName(sha, new Date("2026-11-20T04:30:58Z")));
  });
});

function git(root: string, ...args: string[]): string {
  const done = Bun.spawnSync(["git", ...args], { cwd: root });
  if (done.exitCode !== 0) throw new Error(`git ${args.join(" ")}: ${done.stderr.toString()}`);
  return done.stdout.toString().trim();
}

function commit(root: string, path: string, text: string): string {
  mkdirSync(dirname(join(root, path)), { recursive: true });
  writeFileSync(join(root, path), text);
  git(root, "add", "-A");
  git(root, "commit", "--quiet", "-m", path);
  return git(root, "rev-parse", "HEAD");
}

/** A repository on `dev` with three branches: two that merge, and one that changes the same line as the first. */
function repository(): { root: string; tips: Tip[] } {
  const root = mkdtempSync(join(tmpdir(), "anima-train-"));
  git(root, "init", "--quiet", "--initial-branch=dev");
  git(root, "config", "user.name", "test");
  git(root, "config", "user.email", "test@anima.invalid");
  commit(root, "shared.txt", "one\n");
  const tips = [["hub/a", "shared.txt", "a\n"], ["ui/clash", "shared.txt", "clash\n"], ["docs/c", "c.txt", "c\n"]].map(([branch = "", path = "", text = ""]) => {
    git(root, "switch", "--quiet", "-c", branch, "dev");
    return { branch, sha: commit(root, path, text) };
  });
  git(root, "switch", "--quiet", "dev");
  return { root, tips };
}

describe("R-252 the commit of a train", () => {
  const finish = (root: string) => (tip: Tip) => commitThenCheck(root, `Merge origin/${tip.branch} into dev\n`, () => undefined);

  test("the branches are merged one on the other in their order, and a branch with a conflict is left out", () => {
    const { root, tips } = repository();
    const base = git(root, "rev-parse", "HEAD");
    const made = stack(root, tips, finish(root));
    expect(made.merged.map((tip) => tip.branch)).toEqual(["hub/a", "docs/c"]);
    expect(made.leftOut).toEqual([{ branch: "ui/clash", reason: "a conflict in shared.txt" }]);
    expect(git(root, "status", "--porcelain")).toBe("");
    expect(members(root, base, "HEAD")).toEqual([tips[0], tips[2]] as Tip[]);
    expect(git(root, "merge-base", "--is-ancestor", base, "HEAD")).toBe("");
  }, 60_000);

  test("a branch whose document gates fail on its merge commit is left out, and the next branch still joins", () => {
    const { root, tips } = repository();
    const failing = (tip: Tip): string | undefined =>
      commitThenCheck(root, `Merge origin/${tip.branch} into dev\n`, () => (tip.branch === "hub/a" ? "documents" : undefined));
    const made = stack(root, [tips[0], tips[2]] as Tip[], failing);
    expect(made.merged.map((tip) => tip.branch)).toEqual(["docs/c"]);
    expect(made.leftOut).toEqual([{ branch: "hub/a", reason: "documents" }]);
  }, 60_000);
});
