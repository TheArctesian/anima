import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { conflictsForAHand, gatesRefusal, mergeMessage, parseMergeRule } from "../src/merge.ts";

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

describe("the two rules that wait for the operator (quality/merge-rule.json)", () => {
  test("the rule file of the repository has the two rules off", () => {
    const text = readFileSync(join(import.meta.dir, "..", "merge-rule.json"), "utf8");
    expect(parseMergeRule(text)).toEqual({ fullGates: "mac", mergeWithoutCheck: false });
  });

  test("a rule file that is not exact is refused, and never read as on", () => {
    for (const text of ["", "[]", "null", "{}", '{"fullGates":"github"}', '{"fullGates":"GitHub","mergeWithoutCheck":true}', '{"fullGates":"github","mergeWithoutCheck":"true"}']) {
      expect(typeof parseMergeRule(text), text).toBe("string");
    }
  });

  test("a merge with no check needs the gates on GitHub", () => {
    expect(parseMergeRule('{"fullGates":"mac","mergeWithoutCheck":true}')).toContain("needs fullGates");
    expect(parseMergeRule('{"fullGates":"github","mergeWithoutCheck":true}')).toEqual({ fullGates: "github", mergeWithoutCheck: true });
  });

  test("the gates on GitHub permit a merge only when the job `all` passed", () => {
    const run = (status: string, conclusion: string | null, name = "all") => ({ name, status, conclusion });
    expect(gatesRefusal([run("completed", "success"), run("completed", "success", "go")])).toBeUndefined();
    expect(gatesRefusal([run("completed", "success", "gates / all")])).toBeUndefined();
    expect(gatesRefusal([])).toContain("no run");
    expect(gatesRefusal([run("completed", "success", "go")])).toContain("no run");
    expect(gatesRefusal([run("in_progress", null)])).toContain("in_progress");
    expect(gatesRefusal([run("completed", "failure")])).toContain("failure");
    expect(gatesRefusal([run("completed", "success"), run("completed", "cancelled")])).toContain("cancelled");
  });
});
