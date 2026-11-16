import { describe, expect, test } from "bun:test";
import { checkDocuments, definedIdentifiers, linkTargets, parseKnown, type Document } from "../src/documents/documents.ts";

const REGISTER = "docs/plan/register.md";
const INDEX = "docs/work-items/README.md";
const PLAN_TEXT = "# Plan\n\n| Status | Draft |\n";

function check(documents: Document[], known: string[] = []) {
  const paths = new Set(documents.map((document) => document.path));
  const exists = (path: string) => paths.has(path) || [...paths].some((other) => other.startsWith(`${path}/`));
  return checkDocuments(documents, exists, new Set(known)).map((v) => `${v.path}: ${v.signal} ${v.detail}`);
}

describe("PLN-017 links", () => {
  test("a link to a file or a folder that exists passes, with or without an anchor", () => {
    expect(
      check([
        { path: "docs/work-plan.md", text: "[a](plan/rules.md#pln) [b](plan/) [c](https://example.org) [d](#top)" },
        { path: "docs/plan/rules.md", text: "[up](../work-plan.md)" },
      ]),
    ).toEqual([]);
  });

  test("a link to a file that does not exist fails and names the target", () => {
    expect(check([{ path: "docs/work-plan.md", text: "[gone](plan/gone.md)" }])).toEqual([
      "docs/work-plan.md: broken-link docs/plan/gone.md",
    ]);
  });

  test("a link that leaves the repository fails", () => {
    expect(check([{ path: "AGENTS.md", text: "[out](../other/file.md)" }])).toEqual(["AGENTS.md: broken-link ../other/file.md"]);
  });

  test("a target is a repository path", () => {
    expect(linkTargets({ path: "docs/programme/proposals/jobs.md", text: "[k](../../../contracts/tunnel.md)" })).toEqual([
      "contracts/tunnel.md",
    ]);
  });
});

describe("PLN-001 PLN-012 lists", () => {
  test("a plan, a proposal and a task breakdown that the register names pass", () => {
    expect(
      check([
        { path: REGISTER, text: "[a](../compute-plan.md) [b](../programme/proposals/jobs.md) [c](../research/one-click/s1-link-tasks.md)" },
        { path: "docs/compute-plan.md", text: PLAN_TEXT },
        { path: "docs/programme/proposals/jobs.md", text: PLAN_TEXT },
        { path: "docs/research/one-click/s1-link-tasks.md", text: PLAN_TEXT },
      ]),
    ).toEqual([]);
  });

  test("a proposal that the register does not name fails", () => {
    expect(
      check([
        { path: REGISTER, text: "" },
        { path: "docs/programme/proposals/new-idea.md", text: PLAN_TEXT },
      ]),
    ).toEqual(["docs/programme/proposals/new-idea.md: plan-not-in-register no link in docs/plan/register.md"]);
  });

  test("a plan with no status in its header fails", () => {
    expect(
      check([
        { path: REGISTER, text: "[a](../compute-plan.md)" },
        { path: "docs/compute-plan.md", text: "# Plan\n" },
      ]),
    ).toEqual(["docs/compute-plan.md: plan-without-status no status and no pointer to the plan register in its first 20 lines"]);
  });

  test("a plan that points to the plan register passes", () => {
    expect(
      check([
        { path: REGISTER, text: "[a](../compute-plan.md)" },
        { path: "docs/compute-plan.md", text: "# Plan\n\nProgress: the plan register holds the state of this plan.\n" },
      ]),
    ).toEqual([]);
  });

  test("the work plan and a report are not plans", () => {
    expect(
      check([
        { path: REGISTER, text: "" },
        { path: "docs/work-plan.md", text: "" },
        { path: "docs/research/motion/phase-1.md", text: "" },
      ]),
    ).toEqual([]);
  });

  test("a work item that the index does not name fails", () => {
    expect(
      check([
        { path: INDEX, text: "[C1](compute/C1.md)" },
        { path: "docs/work-items/compute/C1.md", text: "" },
        { path: "docs/work-items/compute/C2.md", text: "" },
      ]),
    ).toEqual(["docs/work-items/compute/C2.md: work-item-not-in-index no link in docs/work-items/README.md"]);
  });
});

describe("PLN-016 one owner for an identifier", () => {
  const first = { path: "docs/compute-plan.md", text: `${PLAN_TEXT}- **CMP-003.** The node must report.` };
  const second = { path: "docs/design-spec.md", text: `${PLAN_TEXT}**CMP-003.** A second text.\nSee CMP-003 and **CMP-004**.` };
  const register = { path: REGISTER, text: "[a](../compute-plan.md) [b](../design-spec.md)" };

  test("an identifier that two documents define fails at the second document", () => {
    expect(check([register, first, second])).toEqual([
      "docs/design-spec.md: identifier-defined-twice CMP-003 is also in docs/compute-plan.md",
    ]);
  });

  test("a known duplicate passes", () => {
    expect(check([register, first, second], ["CMP-003"])).toEqual([]);
  });

  test("a definition is a bold identifier with a full stop at the start of a line", () => {
    expect(definedIdentifiers("- **DEV-001.** a\n**OD-W1.** b\ntext **DEV-002.** c\n- **PH-E-000.** d\n- **DEV-003** e")).toEqual([
      "DEV-001",
      "PH-E-000",
    ]);
  });

  test("the known list has one identifier for each line and permits comments", () => {
    expect([...parseKnown("# reason\nMET-011\n\nVPN-012 # restated\n")]).toEqual(["MET-011", "VPN-012"]);
  });
});
