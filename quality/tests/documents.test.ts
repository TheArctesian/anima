import { describe, expect, test } from "bun:test";
import { checkDocuments, definedIdentifiers, linkTargets, parseKnown, type Document } from "../src/documents/documents.ts";
import { registerRows, withRows } from "../src/documents/register.ts";

const REGISTER = "docs/plan/register.md";
const INDEX = "docs/work-items/README.md";
const PLAN_TEXT = "---\nkind: Plan\nstate: Draft\n---\n\n# Plan\n";

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
    expect(linkTargets({ path: "docs/proposals/jobs.md", text: "[k](../../contracts/tunnel.md)" })).toEqual([
      "contracts/tunnel.md",
    ]);
  });
});

describe("PLN-001 PLN-012 lists", () => {
  test("a plan, a proposal and a task breakdown that the register names pass", () => {
    expect(
      check([
        { path: REGISTER, text: "[a](../plans/compute-plan.md) [b](../proposals/jobs.md) [c](../plans/one-click/s1-link-tasks.md)" },
        { path: "docs/plans/compute-plan.md", text: PLAN_TEXT },
        { path: "docs/proposals/jobs.md", text: PLAN_TEXT },
        { path: "docs/plans/one-click/s1-link-tasks.md", text: PLAN_TEXT },
      ]),
    ).toEqual([]);
  });

  test("a proposal that the register does not name fails", () => {
    expect(
      check([
        { path: REGISTER, text: "" },
        { path: "docs/proposals/new-idea.md", text: PLAN_TEXT },
      ]),
    ).toEqual(["docs/proposals/new-idea.md: plan-not-in-register no link in docs/plan/register.md"]);
  });

  test("a plan with no kind and state in its header fails", () => {
    expect(
      check([
        { path: REGISTER, text: "[a](../plans/compute-plan.md)" },
        { path: "docs/plans/compute-plan.md", text: "# Plan\n\n| Status | Draft |\n" },
      ]),
    ).toEqual(["docs/plans/compute-plan.md: plan-without-state its header needs `kind:` and a `state:` that starts with a word of PLN-007"]);
  });

  test("a state that is not a word of PLN-007 fails", () => {
    expect(
      check([
        { path: REGISTER, text: "[a](../plans/compute-plan.md)" },
        { path: "docs/plans/compute-plan.md", text: "---\nkind: Plan\nstate: Nearly there\n---\n" },
      ]).map((line) => line.split(" ").slice(0, 2).join(" ")),
    ).toEqual(["docs/plans/compute-plan.md: plan-without-state"]);
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

describe("PLN-020 the register is generated", () => {
  const plan = {
    path: "docs/proposals/jobs.md",
    text: "---\nkind: Proposal\nstate: Draft\nevidence: See [the spec](../spec/design-spec.md).\n---\n",
  };
  const spec = { path: "docs/spec/design-spec.md", text: "---\nkind: Specification\nstate: Approved\n---\n" };
  const rows = registerRows([plan, spec]);

  test("a row comes from the header, in the order of the kinds, with links from the register", () => {
    expect(rows.split("\n").filter((line) => line.startsWith("| ["))).toEqual([
      "| [spec/design-spec.md](../spec/design-spec.md) | Specification | Approved |  |",
      "| [proposals/jobs.md](../proposals/jobs.md) | Proposal | Draft | See [the spec](../spec/design-spec.md). |",
    ]);
  });

  test("a register that agrees with the headers passes, and one that does not fails", () => {
    const current = { path: REGISTER, text: `intro\n${rows}\nend` };
    expect(check([current, plan, spec])).toEqual([]);
    const stale = { path: REGISTER, text: current.text.replace("| Draft |", "| Done |") };
    expect(check([stale, plan, spec])).toEqual(["docs/plan/register.md: register-out-of-date run `bun run quality:register`"]);
  });

  test("a register with no generated list is left alone", () => {
    expect(withRows("no markers", rows)).toBeUndefined();
  });
});

describe("PLN-019 mockups", () => {
  test("a mockup that the index does not name fails", () => {
    expect(
      check([
        { path: "docs/mockup/README.md", text: "[a](index.html) [screen](../../apps/client/src/routes/devices)" },
        { path: "docs/mockup/index.html", text: "" },
        { path: "docs/mockup/mobile/index.html", text: "" },
        { path: "apps/client/src/routes/devices/+page.svelte", text: "" },
      ]),
    ).toEqual(["docs/mockup/mobile/index.html: mockup-not-in-index no link in docs/mockup/README.md"]);
  });
});

describe("PLN-016 one owner for an identifier", () => {
  const first = { path: "docs/plans/compute-plan.md", text: `${PLAN_TEXT}- **CMP-003.** The node must report.` };
  const second = { path: "docs/spec/design-spec.md", text: `${PLAN_TEXT}**CMP-003.** A second text.\nSee CMP-003 and **CMP-004**.` };
  const register = { path: REGISTER, text: "[a](../plans/compute-plan.md) [b](../spec/design-spec.md)" };

  test("an identifier that two documents define fails at the second document", () => {
    expect(check([register, first, second])).toEqual([
      "docs/spec/design-spec.md: identifier-defined-twice CMP-003 is also in docs/plans/compute-plan.md",
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
