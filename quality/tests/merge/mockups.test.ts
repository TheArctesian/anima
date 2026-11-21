import { describe, expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { FIELDS, mockupTable, recordCells, recordText, withMockupTable, type Cells } from "../../src/documents/mockup-records.ts";
import { checkMockups, MOCKUPS } from "../../src/documents/mockups.ts";
import { layoutChangedSince } from "../../src/documents/screens.ts";
import { settleGenerated } from "../../src/merge/finish.ts";
import { newerRecord, recordsFromTable, settleMockups } from "../../src/merge/mockups.ts";
import { conflictsForAHand } from "../../src/merge/rules.ts";

const SCREEN = "apps/client/src/lib/projects";
const RECORD = "docs/mockup/records/projects-v2.md";

function git(root: string, ...args: string[]): string {
  const done = Bun.spawnSync(["git", "-c", "user.name=test", "-c", "user.email=test@anima.invalid", ...args], { cwd: root });
  if (done.exitCode !== 0) throw new Error(`git ${args.join(" ")}: ${done.stderr.toString()}`);
  return done.stdout.toString().trim();
}

function write(root: string, path: string, text: string): void {
  mkdirSync(dirname(join(root, path)), { recursive: true });
  writeFileSync(join(root, path), text);
}

function commit(root: string, files: Record<string, string>, message: string): string {
  for (const [path, text] of Object.entries(files)) write(root, path, text);
  git(root, "add", "-A");
  git(root, "commit", "--quiet", "-m", message);
  return git(root, "rev-parse", "--short=8", "HEAD");
}

function cells(compared: string, differences = "None"): Cells {
  return { Mockup: "[projects-v2/index.html](projects-v2/index.html)", Owner: "plan", Screens: `[projects](../../${SCREEN})`, State: "Built", Compared: `\`${compared}\``, Differences: differences };
}

/** The index in the form before the records: one table row. */
function oldIndex(row: Cells): string {
  return `# Mockups\n\n## Index\n\n| ${FIELDS.join(" | ")} |\n| --- | --- | --- | --- | --- | --- |\n| ${FIELDS.map((field) => row[field]).join(" | ")} |\n`;
}

/** The index in the generated form, from the records given. */
function newIndex(records: { path: string; text: string }[]): string {
  return `# Mockups\n\n## Index\n\n${mockupTable(records)}\n`;
}

/** What the merge does after the settle step: the table is written again from the records, then the check runs on the commit. */
function finish(root: string): string | undefined {
  const record = { path: RECORD, text: readFileSync(join(root, RECORD), "utf8") };
  write(root, MOCKUPS, withMockupTable(readFileSync(join(root, MOCKUPS), "utf8"), mockupTable([record])) ?? "");
  git(root, "add", "-A");
  git(root, "commit", "--quiet", "-m", "merge");
  const index = { path: MOCKUPS, text: readFileSync(join(root, MOCKUPS), "utf8") };
  return checkMockups([index, record], (screen, at) => layoutChangedSince(root, screen, at))[0]?.signal;
}

/** Starts a merge that is expected to stop on conflicts; fails if git did not start it at all. */
function mergeStopped(root: string, branch: string): void {
  const done = Bun.spawnSync(["git", "merge", "--no-ff", "--no-commit", branch], { cwd: root });
  if (Bun.spawnSync(["git", "rev-parse", "-q", "--verify", "MERGE_HEAD"], { cwd: root }).exitCode !== 0) {
    throw new Error(`git merge ${branch} did not start: ${done.stderr.toString()}`);
  }
}

function conflicted(root: string): string[] {
  return git(root, "diff", "--name-only", "--diff-filter=U").split("\n").filter((path) => path !== "");
}

function repository(): string {
  const root = mkdtempSync(join(tmpdir(), "anima-mockups-"));
  git(root, "init", "--quiet", "--initial-branch=dev");
  // The identity lives in the repository, so a runner with no global git identity can merge too.
  git(root, "config", "user.name", "test");
  git(root, "config", "user.email", "test@anima.invalid");
  return root;
}

describe("PLN-019 the records of the mockups at a merge", () => {
  test("a branch with the old table merges into a dev with records: its comparison becomes the record, with no hand", () => {
    const root = repository();
    const first = commit(root, { [`${SCREEN}/List.svelte`]: "<ul></ul>\n" }, "screen");
    commit(root, { [MOCKUPS]: oldIndex(cells(first)) }, "index");
    git(root, "switch", "--quiet", "-c", "ui/projects");
    const built = commit(root, { [`${SCREEN}/List.svelte`]: "<ul><li></li></ul>\n" }, "screen again");
    commit(root, { [MOCKUPS]: oldIndex(cells(built, "The tiles stay.")) }, "compared");
    git(root, "switch", "--quiet", "dev");
    const record = { path: RECORD, text: recordText(cells(first)) };
    commit(root, { [RECORD]: record.text, [MOCKUPS]: newIndex([record]) }, "records");

    mergeStopped(root, "ui/projects");
    expect(conflicted(root)).toEqual([MOCKUPS]);
    expect(conflictsForAHand(conflicted(root))).toEqual([]);
    expect(settleMockups(root).written.sort()).toEqual([MOCKUPS, RECORD]);
    expect(recordCells(readFileSync(join(root, RECORD), "utf8"))?.Compared).toBe(`\`${built}\``);
    expect(finish(root)).toBeUndefined();
    expect(readFileSync(join(root, MOCKUPS), "utf8")).toContain("The tiles stay.");
  }, 60_000);

  test("a record that both sides changed: the comparison at the later commit is kept, with no hand", () => {
    const root = repository();
    const first = commit(root, { [`${SCREEN}/List.svelte`]: "<ul></ul>\n" }, "screen");
    const start = recordText(cells(first));
    commit(root, { [RECORD]: start, [MOCKUPS]: newIndex([{ path: RECORD, text: start }]) }, "record");
    git(root, "switch", "--quiet", "-c", "ui/words");
    commit(root, { [RECORD]: recordText(cells(first, "Branch.")) }, "a difference");
    git(root, "switch", "--quiet", "dev");
    const later = commit(root, { [`${SCREEN}/Head.svelte`]: "<h2></h2>\n" }, "head");
    commit(root, { [RECORD]: recordText(cells(later, "Dev.")) }, "compared again");

    mergeStopped(root, "ui/words");
    expect(conflicted(root)).toEqual([RECORD]);
    expect(conflictsForAHand(conflicted(root))).toEqual([]);
    settleMockups(root);
    expect(recordCells(readFileSync(join(root, RECORD), "utf8"))?.Differences).toBe("Dev.");
    expect(finish(root)).toBeUndefined();
  }, 60_000);

  test("the record with the comparison at the later commit wins; with no order the incoming one wins", () => {
    const at = (commit: string) => recordText(cells(commit));
    const chain = (a: string, b: string) => a === "aaaaaaa" && b === "bbbbbbb";
    expect(newerRecord(at("bbbbbbb"), at("aaaaaaa"), chain)).toBe(at("bbbbbbb"));
    expect(newerRecord(at("aaaaaaa"), at("bbbbbbb"), chain)).toBe(at("bbbbbbb"));
    expect(newerRecord(at("ccccccc"), at("ddddddd"), () => false)).toBe(at("ddddddd"));
    expect(newerRecord(at("ccccccc"), recordText({ ...cells("x"), Compared: "Not compared" }), () => false)).toBe(at("ccccccc"));
  });

  test("only the rows that a branch changed or added after the base become records", () => {
    const base = oldIndex(cells("aaaaaaa"));
    expect([...recordsFromTable(base, base).keys()]).toEqual([]);
    expect([...recordsFromTable(oldIndex(cells("bbbbbbb")), base).keys()]).toEqual([RECORD]);
  });
});

const DEVICES = "docs/mockup/records/devices.md";

/** A record of a second mockup, for a branch that adds a row of its own. */
function devicesRecord(compared: string): { path: string; text: string } {
  return { path: DEVICES, text: recordText({ ...cells(compared), Mockup: "[devices/index.html](devices/index.html)" }) };
}

/** The generated index with a line that a person wrote above the table. */
function indexWith(intro: string, records: { path: string; text: string }[]): string {
  return `# Mockups\n\n${intro}\n\n## Index\n\n${mockupTable(records)}\n`;
}

describe("PLN-019 the index of the mockups at a merge: the table is generated, the rest merges", () => {
  test("two branches that each add a record merge with no hand, and a line that the branch wrote is kept", () => {
    const root = repository();
    const first = commit(root, { [`${SCREEN}/List.svelte`]: "<ul></ul>\n", [MOCKUPS]: indexWith("Intro.", []) }, "start");
    git(root, "switch", "--quiet", "-c", "ui/devices");
    const devices = devicesRecord(first);
    commit(root, { [DEVICES]: devices.text, [MOCKUPS]: indexWith("Intro, written again.", [devices]) }, "devices");
    git(root, "switch", "--quiet", "dev");
    const projects = { path: RECORD, text: recordText(cells(first)) };
    commit(root, { [RECORD]: projects.text, [MOCKUPS]: indexWith("Intro.", [projects]) }, "projects");

    mergeStopped(root, "ui/devices");
    expect(conflicted(root)).toEqual([MOCKUPS]);
    expect(settleGenerated(root)).toBeUndefined();
    const index = readFileSync(join(root, MOCKUPS), "utf8");
    expect(index).toContain("Intro, written again.");
    expect(withMockupTable(index, mockupTable([devices, projects]))).toContain("devices/index.html");
  }, 60_000);

  test("a line that both sides wrote differently stops the merge: a person solves it", () => {
    const root = repository();
    commit(root, { [MOCKUPS]: indexWith("Intro.", []) }, "start");
    git(root, "switch", "--quiet", "-c", "docs/intro");
    commit(root, { [MOCKUPS]: indexWith("Intro of the branch.", []) }, "branch");
    git(root, "switch", "--quiet", "dev");
    commit(root, { [MOCKUPS]: indexWith("Intro of dev.", []) }, "dev");

    mergeStopped(root, "docs/intro");
    expect(settleMockups(root).conflict).toBe(`a conflict in the part of ${MOCKUPS} that a person writes`);
    expect(settleGenerated(root)).toBe(`a conflict in the part of ${MOCKUPS} that a person writes`);
  }, 60_000);

  test("a conflict in any other file is not settled", () => {
    const root = repository();
    commit(root, { "crates/hub/src/lib.rs": "a\n", [MOCKUPS]: indexWith("Intro.", []) }, "start");
    git(root, "switch", "--quiet", "-c", "hub/x");
    commit(root, { "crates/hub/src/lib.rs": "b\n" }, "branch");
    git(root, "switch", "--quiet", "dev");
    commit(root, { "crates/hub/src/lib.rs": "c\n" }, "dev");

    mergeStopped(root, "hub/x");
    expect(settleGenerated(root)).toBe("a conflict in crates/hub/src/lib.rs");
  }, 60_000);
});
