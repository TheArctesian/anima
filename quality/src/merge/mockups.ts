/**
 * The records of the mockups at a merge into `dev` (PLN-019). A record that both sides changed
 * keeps the newer comparison; a side whose index still has the table of before the records
 * gives each row that it changed as a record. Then `bun run quality:register` writes the table,
 * and the document gates on the merge commit check the comparison that was kept.
 */
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { isGenerated, isRecord, MOCKUPS, recordCells, recordPath, recordText, tableRows } from "../documents/mockup-records.ts";

const COMMIT = /`([0-9a-f]{7,40})`/;

function comparedAt(record: string): string | undefined {
  return COMMIT.exec(recordCells(record)?.Compared ?? "")?.[1];
}

/**
 * The record with the newer comparison. `older(a, b)` says if commit `a` is an ancestor of
 * commit `b`. When neither is older, or the commits are the same, `incoming` wins: the gates on
 * the merge commit then say if its screen changed after it.
 */
export function newerRecord(current: string, incoming: string, older: (a: string, b: string) => boolean): string {
  const now = comparedAt(current);
  const next = comparedAt(incoming);
  if (next === undefined) return now === undefined ? incoming : current;
  if (now === undefined || now === next) return incoming;
  return older(next, now) ? current : incoming;
}

/** The record of each row that `index` (the table of before the records) changed or added after `base`. */
export function recordsFromTable(index: string, base: string): Map<string, string> {
  const before = new Set(tableRows(base).map((cells) => JSON.stringify(cells)));
  const records = new Map<string, string>();
  for (const cells of tableRows(index)) {
    const path = recordPath(cells.Mockup);
    if (path !== undefined && !before.has(JSON.stringify(cells))) records.set(path, recordText(cells));
  }
  return records;
}

function git(root: string, args: string[]): { ok: boolean; out: string } {
  const done = Bun.spawnSync(["git", ...args], { cwd: root, stdout: "pipe", stderr: "pipe" });
  return { ok: done.exitCode === 0, out: done.stdout.toString() };
}

/**
 * Settles the mockups of the merge that is in progress in `root`: the conflicted records, the
 * rows of a side whose index has the old table, and the index, which takes the generated form.
 * Returns the paths that it wrote. I/O: git and the files of the worktree.
 */
export function settleMockups(root: string): string[] {
  const show = (spec: string): string | undefined => {
    const done = git(root, ["show", spec]);
    return done.ok ? done.out : undefined;
  };
  const older = (a: string, b: string): boolean => git(root, ["merge-base", "--is-ancestor", a, b]).ok;
  const base = git(root, ["merge-base", "HEAD", "MERGE_HEAD"]).out.trim();
  const written = new Map<string, string>();
  for (const side of ["HEAD", "MERGE_HEAD"]) {
    const index = show(`${side}:${MOCKUPS}`);
    if (index === undefined || isGenerated(index)) continue;
    for (const [path, incoming] of recordsFromTable(index, show(`${base}:${MOCKUPS}`) ?? "")) {
      const current = written.get(path) ?? show(`${side === "HEAD" ? "MERGE_HEAD" : "HEAD"}:${path}`);
      written.set(path, current === undefined ? incoming : newerRecord(current, incoming, older));
    }
  }
  const conflicted = git(root, ["diff", "--name-only", "--diff-filter=U"]).out.split("\n").filter(isRecord);
  for (const path of conflicted) {
    const [ours, theirs] = [show(`:2:${path}`), show(`:3:${path}`)];
    const kept = ours === undefined ? theirs : theirs === undefined ? ours : newerRecord(ours, theirs, older);
    if (kept !== undefined) written.set(path, kept);
  }
  const index = [show(`HEAD:${MOCKUPS}`), show(`MERGE_HEAD:${MOCKUPS}`)].find((each) => each !== undefined && isGenerated(each));
  if (index !== undefined) written.set(MOCKUPS, index);
  for (const [path, text] of written) {
    writeFileSync(join(root, path), text);
    git(root, ["add", "--", path]);
  }
  return [...written.keys()];
}
