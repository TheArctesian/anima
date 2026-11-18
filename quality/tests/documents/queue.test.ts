import { describe, expect, test } from "bun:test";
import type { Document } from "../../src/documents/documents.ts";
import { queueRows } from "../../src/documents/queue.ts";
import { check } from "./kit.ts";

describe("PLN-023 the operator queue and the tasks for an agent", () => {
  const priorities = (waits: string) => ({
    path: "docs/plan/priorities.md",
    text: `| Row | Tier | Work | Category | State | Waits for |\n| --- | --- | --- | --- | --- | --- |\n| 4.1 | 0 | The engine | See | In progress | ${waits} |\n`,
  });
  const plan = { path: "docs/plans/compute-plan.md", text: "---\nkind: Plan\nstate: Draft\nevidence: none\n---\n\n| Number | Decision | Result |\n| --- | --- | --- |\n| OD-C1 | Which store? | Postgres. |\n| OD-C2 | Which port? | **Decided: 80.** |\n| OD-C3 | Is the port closed when the client is closed? | Not decided. Yes is better. |\n| D-1 | Which name? | Anima. |\n" };
  const only = (documents: Document[]) => check(documents).filter((line) => line.includes("agent-task") || line.includes("queue"));

  test("a task for an agent needs a link, and its document has a state that permits a build", () => {
    const approved = { ...plan, text: plan.text.replace("state: Draft", "state: Approved") };
    expect(only([priorities("Agent: C1 of [the plan](../plans/compute-plan.md). Operator: OD-C1"), approved])).toEqual([]);
    expect(only([priorities("Operator: approval. Agent: nothing before it")])).toEqual([]);
    expect(only([priorities("Agent: build the model service")])).toEqual(["docs/plan/priorities.md: agent-task-without-link row 4.1"]);
    const proposal = (state: string) => ({ path: "docs/proposals/update.md", text: `---\nkind: Proposal\nstate: ${state}\nevidence: none\n---\n` });
    const inProposal = priorities("Agent: U3 of [the update](../proposals/update.md#tasks)");
    expect(only([inProposal, proposal("Draft")])).toEqual([
      "docs/plan/priorities.md: agent-task-in-unapproved-document row 4.1: docs/proposals/update.md is Draft",
    ]);
    // PLN-007: a ruling can name a task of a proposal whose direction is decided.
    expect(only([inProposal, proposal("Direction decided")])).toEqual([]);
    expect(only([priorities("Agent: None. Operator: approval")])).toEqual([]);
  });

  test("the queue has the rows that wait for the operator, the open decisions and the drafts", () => {
    const rows = queueRows([priorities("Operator: OD-C1"), plan]);
    expect(rows).toContain("| 4.1 | The engine | Operator: OD-C1 |");
    expect(rows).toContain("| OD-C1 | Which store? | Postgres. | [plans/compute-plan.md](plans/compute-plan.md) |");
    // A question with the word closed, and a result that says Not decided, are open.
    expect(rows).toContain("| OD-C3 | Is the port closed when the client is closed? | Not decided. Yes is better. |");
    expect(rows).not.toContain("OD-C2");
    expect(rows).toContain("| [plans/compute-plan.md](plans/compute-plan.md) | Plan | Draft |");
  });

  test("a queue that does not agree with the documents fails", () => {
    const documents = [priorities("Operator: OD-C1"), plan];
    const queue = { path: "docs/operator-queue.md", text: queueRows(documents) };
    expect(only([queue, ...documents])).toEqual([]);
    expect(only([{ ...queue, text: queue.text.replace("Which store?", "Which shop?") }, ...documents])).toEqual([
      "docs/operator-queue.md: queue-out-of-date run `bun run quality:register`",
    ]);
  });

});

describe("PLN-023 what the queue lists", () => {
  const asking = (id = "D-1") => ({ path: "docs/plans/one-click/s1-link-tasks.md", text: `| ID | Question | Recommendation |\n| --- | --- | --- |\n| ${id} | Which name? | Anima. |\n` });
  const plan = { path: "docs/plans/compute-plan.md", text: "| ID | Decision | Reason |\n| --- | --- | --- |\n| D-1 | Which name? | Anima. |\n" };
  const priorities = (waits: string) => ({
    path: "docs/plan/priorities.md",
    text: `| Row | Tier | Work | Category | State | Waits for |\n| --- | --- | --- | --- | --- | --- |\n| 4.1 | 0 | The engine | See | In progress | ${waits} |\n`,
  });
  const only = (documents: Document[]) => check(documents).filter((line) => line.includes("agent-task") || line.includes("queue"));

  test("a recorded decision is not in the queue, and a question that waits for the operator is", () => {
    expect(queueRows([plan])).not.toContain("D-1");
    expect(queueRows([asking()])).toContain("| D-1 | Which name? | Anima. | [plans/one-click/s1-link-tasks.md](plans/one-click/s1-link-tasks.md) |");
    const questions = { path: "docs/proposals/power-health.md", text: "| ID | Question | Recommendation |\n| --- | --- | --- |\n| Q1 | Which limit? | Ten. |\n" };
    expect(queueRows([questions])).toContain("| Q1 | Which limit? | Ten. |");
  });

  test("the same number in two documents gives two rows", () => {
    const other = { ...asking(), path: "docs/plans/one-click/s2-vnc-tasks.md" };
    const rows = queueRows([asking(), other]).split("\n").filter((line) => line.startsWith("| D-1 |"));
    expect(rows).toHaveLength(2);
  });

  test("an older check is open only when its last cell is", () => {
    const checks = (state: string) => ({
      path: "docs/need-human-testing.md",
      text: `## Checks that other documents hold\n\n| Check | State |\n| --- | --- |\n| Open the door | ${state} |\n`,
    });
    expect(queueRows([checks("Passed")])).not.toContain("Open the door");
    expect(queueRows([checks("Open. It waits for a device.")])).toContain("| Open the door | Open. It waits for a device. |");
  });

  test("every Agent clause of a row needs a link, and a lower-case word is prose", () => {
    expect(only([priorities("Agent: A1 of [the plan](../plans/compute-plan.md). Agent: build the service")])).toEqual([
      "docs/plan/priorities.md: agent-task-without-link row 4.1",
    ]);
    expect(only([priorities("Operator: approval; the agent: builds it")])).toEqual([]);
  });

  test("a queue or a register without markers fails with its signal", () => {
    expect(only([{ path: "docs/operator-queue.md", text: "no markers" }])).toEqual([
      "docs/operator-queue.md: queue-without-list the two marker comments of the generated part are missing",
    ]);
  });
});
