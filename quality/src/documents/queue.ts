import { posix } from "node:path";
import type { Violation } from "../report.ts";
import type { Document } from "./documents.ts";
import { header } from "./register.ts";

/** The list of what waits for the operator: generated, in one place (PLN-023). */
export const QUEUE = "docs/operator-queue.md";
const PRIORITIES = "docs/plan/priorities.md";
const CHECKS = "docs/need-human-testing.md";
const START = "<!-- generated from the planning documents: bun run quality:register -->";
const END = "<!-- end of the generated queue -->";
const LINK = /\[([^\]]*)\]\(([^)\s]+)\)/g;
const DECISION = /^(?:OD-(?:[A-Z]+-?)?\d+|HM-D\d+)$/;
const SETTLED = /\bdecided\b|not a decision|withdrawn|closed|superseded/i;
const WAITS = /^(?:Draft|Direction decided|Not checked)\b/;
const LIMIT = 160;

function cells(row: string): string[] {
  return row.split("|").slice(1, -1).map((cell) => cell.trim());
}

function plain(text: string): string {
  const stripped = text.replace(LINK, "$1").replace(/\*\*/g, "");
  return stripped.length > LIMIT ? `${stripped.slice(0, LIMIT).trimEnd()}…` : stripped;
}

function linked(path: string): string {
  const target = posix.relative(posix.dirname(QUEUE), path);
  return `[${target}](${target})`;
}

/** The rows of the priority order, as [row, work, waits for]. */
function priorityRows(documents: Document[]): string[][] {
  const text = documents.find((document) => document.path === PRIORITIES)?.text ?? "";
  return text.split("\n").filter((line) => /^\| 4\.\d+ \|/.test(line)).map(cells);
}

function ordered(documents: Document[]): string[] {
  return priorityRows(documents)
    .filter((row) => (row.at(-1) ?? "").includes("Operator:"))
    .map((row) => `| ${row[0] ?? ""} | ${row[2] ?? ""} | ${plain(row.at(-1) ?? "")} |`);
}

function decisions(documents: Document[]): string[] {
  const seen = new Set<string>();
  const rows: string[] = [];
  for (const document of documents) {
    for (const line of document.text.split("\n").filter((each) => each.startsWith("|"))) {
      const row = cells(line);
      const id = plain(row[0] ?? "");
      if (!DECISION.test(id) || seen.has(id) || SETTLED.test(line)) continue;
      seen.add(id);
      rows.push(`| ${id} | ${plain(row[1] ?? "")} | ${linked(document.path)} |`);
    }
  }
  return rows;
}

function approvals(documents: Document[]): string[] {
  return documents
    .map((document) => ({ path: document.path, fields: header(document.text) }))
    .filter(({ fields }) => fields.has("kind") && WAITS.test(fields.get("state") ?? ""))
    .map(({ path, fields }) => `| ${linked(path)} | ${fields.get("kind") ?? ""} | ${fields.get("state") ?? ""} |`);
}

function humanChecks(documents: Document[]): string[] {
  const text = documents.find((document) => document.path === CHECKS)?.text ?? "";
  const open = text.slice(text.indexOf("\n## Open"), text.indexOf("\n## Not built yet"));
  return [...open.matchAll(/^### (HT-\d+): (.+)$/gm)].map((match) => `| ${match[1] ?? ""} | ${plain(match[2] ?? "")} |`);
}

function table(title: string, head: string, rows: string[]): string[] {
  const columns = head.split(" | ").length;
  const rule = `| ${Array.from({ length: columns }, () => "---").join(" | ")} |`;
  return [`## ${title} (${rows.length})`, "", `| ${head} |`, rule, ...rows, ""];
}

/** The generated queue: what waits for the operator, from the documents that own each fact. */
export function queueRows(all: Document[]): string {
  // The queue is not a source of itself, and the archive is finished.
  const documents = all
    .filter((each) => each.path.startsWith("docs/") && !each.path.startsWith("docs/archive/") && each.path !== QUEUE)
    .sort((a, b) => a.path.localeCompare(b.path));
  return [
    START,
    "",
    ...table("1. In the order of the priorities", "Row | Work | Waits for", ordered(documents)),
    ...table("2. Open decisions", "Decision | Question | Document", decisions(documents)),
    ...table("3. Documents that wait for an approval", "Document | Kind | State", approvals(documents)),
    ...table("4. Checks that only the operator can do", "Check | What", humanChecks(documents)),
    END,
  ].join("\n");
}

/** `queue` with its generated part replaced; undefined when it has no markers. */
export function withQueue(queue: string, rows: string): string | undefined {
  const start = queue.indexOf(START);
  const end = queue.indexOf(END);
  if (start === -1 || end === -1) return undefined;
  return queue.slice(0, start) + rows + queue.slice(end + END.length);
}

function agentTasks(documents: Document[]): Violation[] {
  const violations: Violation[] = [];
  for (const row of priorityRows(documents)) {
    const clause = /Agent: (?!nothing)([^]*?)(?=Operator:|$)/.exec(row.at(-1) ?? "")?.[1];
    if (clause === undefined) continue;
    const targets = [...clause.matchAll(LINK)].map((match) => posix.normalize(posix.join(posix.dirname(PRIORITIES), match[2] ?? "")));
    if (targets.length === 0) violations.push({ path: PRIORITIES, signal: "agent-task-without-link", detail: `row ${row[0] ?? ""}` });
    for (const target of targets.filter((each) => each.startsWith("docs/proposals/"))) {
      violations.push({ path: PRIORITIES, signal: "agent-task-in-proposal", detail: `row ${row[0] ?? ""}: ${target}` });
    }
  }
  return violations;
}

/** PLN-023: the queue agrees with the documents, and a task for an agent is in a plan. */
export function checkQueue(documents: Document[]): Violation[] {
  const queue = documents.find((document) => document.path === QUEUE);
  const current = queue === undefined ? undefined : withQueue(queue.text, queueRows(documents));
  const stale = queue !== undefined && current !== undefined && current !== queue.text;
  return [...agentTasks(documents), ...(stale ? [{ path: QUEUE, signal: "queue-out-of-date", detail: "run `bun run quality:register`" }] : [])];
}
