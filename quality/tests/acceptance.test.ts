import { describe, expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
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

  test("@pending at the end of the text is caught", () => {
    expect(pendingUses("type A {\n  id: ID! @pending")).toHaveLength(1);
  });

  test("@pending in a description, single-line or block, is prose", () => {
    const schema = ['"uses @pending soon"', "type A {", '  """', "  a @pending note", '  """', "  id: ID!", "}"].join("\n");
    expect(pendingUses(schema)).toEqual([]);
  });

  test("a # inside a string does not hide a later @pending", () => {
    const findings = pendingUses('type A {\n  f(a: String = "x#y"): ID @pending\n}');
    expect(findings.map((finding) => finding.line)).toEqual([2]);
  });

  test("line numbers survive a multi-line block string", () => {
    const schema = ['"""', "one", "two", '"""', "type A @pending {", "}"].join("\n");
    expect(pendingUses(schema).map((finding) => finding.line)).toEqual([5]);
  });

  test("a second @pending on the declaration line is a use", () => {
    expect(pendingUses(`${declaration} @pending`)).toHaveLength(1);
  });

  test("directive @pendingX is not the declaration", () => {
    expect(pendingUses("directive @pendingX on OBJECT")).toEqual([]);
    expect(pendingUses("directive @pending2 on OBJECT @pending")).toHaveLength(1);
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

describe("criterion 24 command line", () => {
  const script = resolve(import.meta.dir, "..", "acceptance", "phase-3.ts");

  function run(schema: string | undefined): { code: number | null; out: string; err: string } {
    const root = mkdtempSync(join(tmpdir(), "acceptance-"));
    mkdirSync(join(root, "contracts"));
    mkdirSync(join(root, "quality"));
    if (schema !== undefined) writeFileSync(join(root, "contracts", "schema.graphql"), schema);
    writeFileSync(join(root, "quality", "baseline.json"), "{}");
    const done = Bun.spawnSync(["bun", script, root]);
    return { code: done.exitCode, out: done.stdout.toString(), err: done.stderr.toString() };
  }

  test("exit 0 and ok when clean", () => {
    const done = run("type A {\n  id: ID!\n}\n");
    expect(done.code).toBe(0);
    expect(done.out).toContain("acceptance phase-3: ok");
  });

  test("exit 1 and file:line: message on a finding", () => {
    const done = run("type A {\n  id: ID! @pending\n}\n");
    expect(done.code).toBe(1);
    expect(done.err).toContain("contracts/schema.graphql:2: @pending is used");
  });

  test("exit 2 when an input is missing", () => {
    const done = run(undefined);
    expect(done.code).toBe(2);
    expect(done.err).toContain("cannot use contracts/schema.graphql");
  });
});
