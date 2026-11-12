import { describe, expect, test } from "bun:test";
import { resolve } from "node:path";
import { AcceptanceInputError, baselineEntries, checkPhase3, pendingUses } from "../acceptance/phase-3.ts";

const declaration = "directive @pending on OBJECT | FIELD_DEFINITION";

describe("CON-002 / P3-002 K1 holds no @pending", () => {
  test("a clean schema passes", () => {
    expect(pendingUses("type A {\n  id: ID!\n}\n")).toEqual([]);
  });

  test("@pending on a field, an enum value and a type is caught with its line", () => {
    const schema = ["type A @pending {", "  id: ID! @pending", "}", "enum E {", "  X @pending(why: 1)", "}"].join("\n");
    expect(pendingUses(schema).map((finding) => finding.line)).toEqual([1, 2, 5]);
  });

  test("the declaration and comments are allowed", () => {
    const schema = [`# marked @pending`, declaration, 'scalar JSON # an @pending note', '"doc"'].join("\n");
    expect(pendingUses(schema)).toEqual([]);
  });

  test("a longer directive name is not matched", () => {
    expect(pendingUses("type A @pendingX {\n  id: ID!\n}")).toEqual([]);
  });

  test("@pending at the end of a line or before a newline is caught", () => {
    expect(pendingUses("type A {\n  id: ID! @pending")).toHaveLength(1);
  });
});

describe("CQ-010 the baseline is empty", () => {
  test("an empty object passes", () => {
    expect(baselineEntries("{}")).toEqual([]);
  });

  test("each remaining key is reported", () => {
    const findings = baselineEntries('{"a.rs": {"file-lines": 1}, "b.ts": {}}');
    expect(findings.map((finding) => finding.message)).toEqual([
      "baseline entry remains: a.rs",
      "baseline entry remains: b.ts",
    ]);
  });

  test("malformed JSON is an input error", () => {
    expect(() => baselineEntries("{")).toThrow(AcceptanceInputError);
    expect(() => baselineEntries("[]")).toThrow(AcceptanceInputError);
  });
});

describe("criterion 24 on this repository", () => {
  test("the repository at this commit passes", () => {
    expect(checkPhase3(resolve(import.meta.dir, "..", ".."))).toEqual([]);
  });

  test("an unreadable repository root is an input error", () => {
    expect(() => checkPhase3("/nonexistent-anima-root")).toThrow(AcceptanceInputError);
  });
});
