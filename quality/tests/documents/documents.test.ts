import { describe, expect, test } from "bun:test";
import { checkDocuments, definedIdentifiers, linkTargets, parseKnown, type Document } from "../../src/documents/documents.ts";
import { registerRows, withRows } from "../../src/documents/register.ts";
import { check } from "./kit.ts";

const REGISTER = "docs/plan/register.md";
const INDEX = "docs/work-items/README.md";
const PLAN_TEXT = "---\nkind: Plan\nstate: Draft\nevidence: none\n---\n\n# Plan\n";

/** The register with the given text outside its generated list, and the list that agrees with `documents`. */
function registered(text: string, documents: Document[]): Document {
  return { path: REGISTER, text: `${text}\n${registerRows(documents)}` };
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
    const plans = [
      { path: "docs/plans/compute-plan.md", text: PLAN_TEXT },
      { path: "docs/proposals/jobs.md", text: PLAN_TEXT },
      { path: "docs/plans/one-click/s1-link-tasks.md", text: PLAN_TEXT },
    ];
    const links = "[a](../plans/compute-plan.md) [b](../proposals/jobs.md) [c](../plans/one-click/s1-link-tasks.md)";
    expect(check([registered(links, plans), ...plans])).toEqual([]);
  });

  test("a proposal that the register does not name fails", () => {
    const idea = { path: "docs/proposals/new-idea.md", text: PLAN_TEXT };
    expect(check([{ path: REGISTER, text: registerRows([]) }, idea])).toEqual([
      "docs/proposals/new-idea.md: plan-not-in-register no link in docs/plan/register.md",
      "docs/plan/register.md: register-out-of-date run `bun run quality:register`",
    ]);
  });

  test("a plan with no kind and state in its header fails", () => {
    const plan = { path: "docs/plans/compute-plan.md", text: "# Plan\n\n| Status | Draft |\n" };
    expect(check([registered("[a](../plans/compute-plan.md)", [plan]), plan])).toEqual(["docs/plans/compute-plan.md: plan-without-state its header needs `kind:` and a `state:` that starts with a word of PLN-007"]);
  });

  test("a state that is not a word of PLN-007 fails", () => {
    const plan = { path: "docs/plans/compute-plan.md", text: "---\nkind: Plan\nstate: Nearly there\n---\n" };
    expect(
      check([registered("[a](../plans/compute-plan.md)", [plan]), plan]).map((line) => line.split(" ").slice(0, 2).join(" ")),
    ).toEqual(["docs/plans/compute-plan.md: plan-without-state"]);
  });

  test("the work plan and a report are not plans", () => {
    const others = [
      { path: "docs/work-plan.md", text: "" },
      { path: "docs/research/motion/phase-1.md", text: "" },
    ];
    expect(check([registered("", others), ...others])).toEqual([]);
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
  const spec = { path: "docs/spec/design-spec.md", text: "---\nkind: Specification\nstate: Approved\nevidence: none\n---\n" };
  const rows = registerRows([plan, spec]);

  test("a row comes from the header, in the order of the kinds, with links from the register", () => {
    expect(rows.split("\n").filter((line) => line.startsWith("| ["))).toEqual([
      "| [spec/design-spec.md](../spec/design-spec.md) | Specification | Approved | none |",
      "| [proposals/jobs.md](../proposals/jobs.md) | Proposal | Draft | See [the spec](../spec/design-spec.md). |",
    ]);
  });

  test("a register that agrees with the headers passes, and one that does not fails", () => {
    const current = { path: REGISTER, text: `intro\n${rows}\nend` };
    expect(check([current, plan, spec])).toEqual([]);
    const stale = { path: REGISTER, text: current.text.replace("| Draft |", "| Done |") };
    expect(check([stale, plan, spec])).toEqual(["docs/plan/register.md: register-out-of-date run `bun run quality:register`"]);
  });

  test("a register with no markers has no generated list, and the check says so", () => {
    expect(withRows("no markers", rows)).toBeUndefined();
    expect(check([{ path: REGISTER, text: "[a](../proposals/jobs.md) [b](../spec/design-spec.md)" }, plan, spec])).toEqual([
      "docs/plan/register.md: register-without-list the two marker comments of the generated list are missing",
    ]);
  });

  test("a plan with a state and no evidence fails", () => {
    expect(check([{ path: "docs/plans/compute-plan.md", text: "---\nkind: Plan\nstate: Done\n---\n" }]).filter((line) => line.includes("evidence"))).toEqual([
      "docs/plans/compute-plan.md: plan-without-evidence its header needs `evidence:` (PLN-009)",
    ]);
  });
});

describe("PLN-021 one number for each ruling", () => {
  test("a number that two rulings have fails; a mention of a number does not", () => {
    const log = "- **R-118 (operator).** First.\n- **CR-26 (orchestrator; R-118).** It names R-118.\n- **R-118 (operator).** Second.\n";
    expect(check([{ path: "docs/rulings.md", text: log }])).toEqual(["docs/rulings.md: ruling-number-used-twice R-118"]);
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
  const register = registered("[a](../plans/compute-plan.md) [b](../spec/design-spec.md)", [first, second]);
  const alone = registered("[a](../plans/compute-plan.md)", [first]);

  test("an identifier that two documents define fails at the second document", () => {
    expect(check([register, first, second])).toEqual([
      "docs/spec/design-spec.md: identifier-defined-twice CMP-003 is also in docs/plans/compute-plan.md",
    ]);
  });

  test("a known duplicate passes, and a known identifier with one owner must leave the list", () => {
    expect(check([register, first, second], ["CMP-003"])).toEqual([]);
    expect(check([alone, first], ["CMP-003"])).toEqual(["quality/duplicate-identifiers.txt: known-duplicate-has-one-owner CMP-003"]);
  });

  test("a definition with a title, and one in a numbered list, are definitions; the archive is not read", () => {
    expect(definedIdentifiers("- **PLN-001. One register.** a\n1. **ROL-002.** b\n- **PLN-002:** a note\n1. **D-1.** a local number")).toEqual(["PLN-001", "ROL-002"]);
    const old = { path: "docs/archive/phase-1/implementation-plan.md", text: "**CMP-003.** old" };
    expect(check([registered("[a](../plans/compute-plan.md)", [first, old]), first, old])).toEqual([]);
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

describe("PLN-022 claims", () => {
  const plan = { path: "docs/plans/compute-plan.md", text: "---\nkind: Plan\nstate: Draft\nevidence: none\n---\n| C1 | Not started |\n| C2 | Not started |\n" };
  const claim = (name: string, task: string, extra = "") => ({
    path: `docs/claims/${name}.md`,
    text: `---\ntask: ${task}\nplan: docs/plans/compute-plan.md\nmachine: studio\nagent: Claude Code\nbranch: compute/c1\nstarted: 2026-11-17\n${extra}---\n`,
  });
  const only = (documents: Document[]) => check(documents).filter((line) => line.includes("claim"));

  test("a complete claim passes, and the README of the folder is not a claim", () => {
    expect(only([plan, claim("c1", "C1"), { path: "docs/claims/README.md", text: "" }])).toEqual([]);
  });

  test("a claim with no machine, or with a plan that does not exist, fails", () => {
    const text = "---\ntask: C1\nplan: docs/plans/gone-plan.md\nagent: Codex\nbranch: b\nstarted: today\n---\n";
    expect(only([{ path: "docs/claims/c1.md", text }])).toEqual([
      "docs/claims/c1.md: claim-not-complete no machine",
      "docs/claims/c1.md: claim-not-complete the plan docs/plans/gone-plan.md does not exist",
      "docs/claims/c1.md: claim-not-complete started is not a date: today",
    ]);
  });

  test("the plan names the task as a word of its own", () => {
    const only10 = { ...plan, text: plan.text.replace("| C1 |", "| C10 |") };
    expect(only([only10, claim("c1", "C1")])).toEqual(["docs/claims/c1.md: claim-not-complete the plan does not name the task C1"]);
  });

  test("a second claim of the same task fails and names the first", () => {
    expect(only([plan, claim("a", "C1"), claim("b", "C1"), claim("c", "C2")])).toEqual([
      "docs/claims/b.md: task-claimed-twice docs/claims/a.md holds the same task",
    ]);
  });
});

describe("PLN-022 one task, whichever plan", () => {
  test("the same task claimed through two plans fails", () => {
    const text = (plan: string) => `---\ntask: C1\nplan: ${plan}\nmachine: studio\nagent: Codex\nbranch: b\nstarted: 2026-11-17\n---\n`;
    const plans = ["docs/plans/a-plan.md", "docs/plans/b-plan.md"].map((path) => ({ path, text: "| C1 |" }));
    const claims = [
      { path: "docs/claims/a.md", text: text(plans[0]?.path ?? "") },
      { path: "docs/claims/b.md", text: text(plans[1]?.path ?? "") },
    ];
    expect(check([...plans, ...claims]).filter((line) => line.includes("task-claimed-twice"))).toEqual([
      "docs/claims/b.md: task-claimed-twice docs/claims/a.md holds the same task",
    ]);
  });
});

describe("PLN-019 a mockup agrees with the application", () => {
  const index = (state: string, compared: string) => ({
    path: "docs/mockup/README.md",
    text: `# Mockups\n\n## Index\n\n| Mockup | Owner | Screens | State | Compared | Differences |\n| --- | --- | --- | --- | --- | --- |\n| [a.html](a.html) | owner | [devices](../../apps/client/src/routes/devices) | ${state} | ${compared} |  |\n`,
  });
  const files = [
    { path: "docs/mockup/a.html", text: "" },
    { path: "apps/client/src/routes/devices/+page.svelte", text: "" },
  ];
  const run = (state: string, compared: string, later: number | undefined) =>
    checkDocuments([index(state, compared), ...files], () => true, new Set(), () => later).map((v) => `${v.signal} ${v.detail}`);

  test("a screen with no commit after the comparison passes, with an empty last cell", () => {
    expect(run("Built", "`abc1234` (2026-11-17)", 0)).toEqual([]);
  });

  test("a screen with a later commit fails and names the commit of the comparison", () => {
    expect(run("**Built**", "`abc1234`", 2)).toEqual([
      "screen-changed-after-mockup apps/client/src/routes/devices has 2 layout file(s) that changed after abc1234; compare [a.html](a.html) again",
    ]);
  });

  test("a commit that the repository does not have fails", () => {
    expect(run("Approved", "`abc1234`", undefined)).toEqual(["mockup-commit-not-found abc1234 of [a.html](a.html)"]);
  });

  test("a built mockup that names a commit and no screen fails", () => {
    const bare = { path: "docs/mockup/README.md", text: index("Built", "`abc1234`").text.replace("[devices](../../apps/client/src/routes/devices)", "none") };
    expect(checkDocuments([bare, files[0] ?? bare], () => true, new Set()).map((v) => v.signal)).toEqual(["mockup-without-screens"]);
  });

  test("a mockup that is not built can wait for its first comparison; a built one cannot, and a day is not a commit", () => {
    expect(run("Approved", "Not compared", 5)).toEqual([]);
    expect(run("built", "2026-11-17", 0)).toEqual(["mockup-not-compared [a.html](a.html)"]);
  });
});
