import { describe, expect, test } from "bun:test";
import { assign, placeholders } from "../src/numbers.ts";

const RULINGS = "- **R-205 (operator).** A ruling.\n- **R-206 (operator).** A ruling.\n";
const CHECKS = "### HT-043: a check\n\n### HT-044: a check\n";

describe("numbers that a merge assigns (PLN-021)", () => {
  test("each placeholder gets the next free number of its series, in each file that names it", () => {
    const done = assign([
      { path: "docs/rulings.md", text: `${RULINGS}- **R-NEW-socket (operator).** See HT-NEW-socket.\n` },
      { path: "docs/need-human-testing.md", text: `${CHECKS}### HT-NEW-socket: by R-NEW-socket\n` },
      { path: "crates/daemon/src/hub.rs", text: "// HT-NEW-socket is the check.\n" },
    ]);
    expect([...done.numbers]).toEqual([["R-NEW-socket", "R-207"], ["HT-NEW-socket", "HT-045"]]);
    expect(done.changed.map((file) => file.text)).toEqual([
      `${RULINGS}- **R-207 (operator).** See HT-045.\n`,
      `${CHECKS}### HT-045: by R-207\n`,
      "// HT-045 is the check.\n",
    ]);
  });

  test("two placeholders of one series get two numbers, in the order of their names", () => {
    const done = assign([{ path: "docs/rulings.md", text: `${RULINGS}R-NEW-b then R-NEW-a then R-NEW-ab\n` }]);
    expect(done.changed[0]?.text).toBe(`${RULINGS}R-209 then R-207 then R-208\n`);
  });

  test("a series keeps its digits, and the work plan decisions have no hyphen before the number", () => {
    const done = assign([
      { path: "a.md", text: "SF-014 SF-NEW-x DEF-005 DEF-NEW-x OD-W39 OD-W-NEW-linux HT-NEW-first" },
    ]);
    expect(done.changed[0]?.text).toBe("SF-014 SF-015 DEF-005 DEF-006 OD-W39 OD-W40 HT-001");
  });

  test("a file whose name is a placeholder gets the number in its name", () => {
    const done = assign([
      { path: "docs/work-items/defects/DEF-005.md", text: "# DEF-005\n" },
      { path: "docs/work-items/defects/DEF-NEW-copy.md", text: "# DEF-NEW-copy\n" },
    ]);
    expect(done.renamed).toEqual([
      { from: "docs/work-items/defects/DEF-NEW-copy.md", to: "docs/work-items/defects/DEF-006.md" },
    ]);
    expect(done.changed).toEqual([{ path: "docs/work-items/defects/DEF-006.md", text: "# DEF-006\n" }]);
  });

  test("a number of a different series, or a word that only looks like a placeholder, is not touched", () => {
    const text = "CR-NEW-x OD-ROL-16 HM-D16 PR-NEW-x R-NEWER R-NEW- XR-NEW-a BLD-045 R-12a";
    expect(placeholders([{ path: "a.md", text }])).toEqual([]);
    expect(assign([{ path: "a.md", text }]).changed).toEqual([]);
  });

  test("with no placeholder nothing changes", () => {
    const done = assign([{ path: "docs/rulings.md", text: RULINGS }]);
    expect([done.numbers.size, done.changed.length, done.renamed.length]).toEqual([0, 0, 0]);
  });
});
