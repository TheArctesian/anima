import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { conflictsForAHand, findingKey, gatesRefusal, headingsTwice, mergeMessage, ownerKey, parseMergeRule, rowsTwice, twiceInOneTable } from "../../src/merge/rules.ts";

describe("the merge of a branch into dev", () => {
  test("PLN-020 a conflict in a generated file needs no hand; each other conflict does", () => {
    expect(conflictsForAHand(["docs/plan/register.md", "docs/operator-queue.md"])).toEqual([]);
    expect(conflictsForAHand(["docs/operator-queue.md", "docs/rulings.md", "crates/hub/src/main.rs"])).toEqual([
      "docs/rulings.md",
      "crates/hub/src/main.rs",
    ]);
  });

  test("PLN-021 the merge commit names each number that it assigned", () => {
    expect(mergeMessage("origin/daemon/w0", "dev", [])).toBe("Merge origin/daemon/w0 into dev\n");
    expect(mergeMessage("origin/daemon/w0", "dev", [["R-NEW-socket", "R-207"], ["HT-NEW-socket", "HT-045"]])).toBe(
      "Merge origin/daemon/w0 into dev\n\nNumbers assigned at this merge (PLN-021):\nR-NEW-socket is R-207\nHT-NEW-socket is HT-045\n",
    );
  });
});

describe("the two rules of a merge (quality/merge-rule.json; R-NEW-gaterule, R-NEW-automerge)", () => {
  test("the rule file of the repository has the two rules on, and the carry-over of documents (OD-W52)", () => {
    const text = readFileSync(join(import.meta.dir, "..", "..", "merge-rule.json"), "utf8");
    expect(parseMergeRule(text)).toEqual({ fullGates: "github", mergeWithoutCheck: true, carryDocuments: true });
  });

  test("a rule file that is not exact is refused, and never read as on", () => {
    for (const text of ["", "[]", "null", "{}", '{"fullGates":"github"}', '{"fullGates":"GitHub","mergeWithoutCheck":true}', '{"fullGates":"github","mergeWithoutCheck":"true"}']) {
      expect(typeof parseMergeRule(text), text).toBe("string");
    }
  });

  test("a train is carried over a move of dev only when the rule says so in exact words", () => {
    const on = parseMergeRule('{"fullGates":"github","mergeWithoutCheck":true,"carryDocuments":true}');
    expect(on).toEqual({ fullGates: "github", mergeWithoutCheck: true, carryDocuments: true });
    const loose = parseMergeRule('{"fullGates":"github","mergeWithoutCheck":true,"carryDocuments":"true"}');
    expect(loose).toEqual({ fullGates: "github", mergeWithoutCheck: true, carryDocuments: false });
  });

  test("a merge with no check needs the gates on GitHub", () => {
    expect(parseMergeRule('{"fullGates":"mac","mergeWithoutCheck":true}')).toContain("needs fullGates");
    expect(parseMergeRule('{"fullGates":"github","mergeWithoutCheck":true}')).toEqual({ fullGates: "github", mergeWithoutCheck: true, carryDocuments: false });
  });

  test("the gates on GitHub permit a merge only when the job `passed` passed", () => {
    const run = (status: string, conclusion: string | null, name = "passed") => ({ name, status, conclusion });
    expect(gatesRefusal([run("completed", "success"), run("completed", "success", "go")])).toBeUndefined();
    expect(gatesRefusal([run("completed", "success", "gates / passed")])).toBeUndefined();
    expect(gatesRefusal([])).toContain("no run");
    expect(gatesRefusal([run("completed", "success", "go")])).toContain("no run");
    expect(gatesRefusal([run("in_progress", null)])).toContain("in_progress");
    expect(gatesRefusal([run("completed", "failure")])).toContain("failure");
    expect(gatesRefusal([run("completed", "success"), run("completed", "cancelled")])).toContain("cancelled");
  });
});

describe("a table that merges by union", () => {
  test("a row that two branches changed is found two times; rows that were only added are not", () => {
    const table = [
      "| Decision | Question | State |",
      "| --- | --- | --- |",
      "| OD-W38 | A question | Open. |",
      "| OD-W39 | A question | Open. |",
      "| OD-W39 | A question | Decided (R-200). |",
      "| OD-W40 | Added by one branch | Open. |",
      "| OD-W-NEW-x | Added by the other branch | Open. |",
      "A line of text with OD-W38 in it.",
    ].join("\n");
    expect(rowsTwice(table)).toEqual(["OD-W39"]);
    expect(rowsTwice("no table")).toEqual([]);
  });

  test("a union merge of the findings list or the owners table that kept two versions of an entry is found", () => {
    const findings = "| SF-001 | a | Open |\n| SF-002 | b | Open |\n| SF-001 | a | Accepted |\n\ntext\n\n| SF-002 | b, in a second table | Open |\n\n### SF-001: a\n\n### SF-002: b\n\n### SF-001: a\n";
    expect(twiceInOneTable(findings, findingKey)).toEqual(["SF-001"]);
    expect(headingsTwice(findings)).toEqual(["SF-001"]);
    const owners = "| Subject | Owner | Identifiers |\n| --- | --- | --- |\n| Jobs | [jobs.md](a) | JOB-001 to JOB-027 |\n| Ideas | [x](b) | None yet |\n| Other | [y](c) | None yet |\n| Jobs and routing | [jobs.md](a) | JOB-001 to JOB-027 |\n";
    expect(twiceInOneTable(owners, ownerKey)).toEqual(["JOB-001 to JOB-027"]);
  });
});
