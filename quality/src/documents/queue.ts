import { posix } from "node:path";
import type { Violation } from "../report.ts";
import type { Document } from "./documents.ts";
import { replaceBlock } from "./generated.ts";
import { header } from "./register.ts";

/** The list of what waits for the operator: generated, in one place (PLN-023). */
export const QUEUE = "docs/operator-queue.md";
const PRIORITIES = "docs/plan/priorities.md";
const CHECKS = "docs/need-human-testing.md";
const START = "<!-- generated from the planning documents: bun run quality:register -->";
const END = "<!-- end of the generated queue -->";
const LINK = /\[([^\]]*)\]\(([^)\s]+)\)/g;
const DECISION = /^(?:OD-(?:[A-Z]+-?)?\d+|HM-D\d+|D-\d+|Q\d+)$/;
/** A number of one document: it waits for the operator only in a table that asks a question. */
const LOCAL = /^(?:D-\d+|Q\d+)$/;
/** A number that is one decision in the whole project; `D-1` and `Q1` are numbers of one document. */
const GLOBAL = /^(?:OD-|HM-D)/;
/** The result of a decision row is its last cell. It is settled only when that cell starts with one of these words. */
const SETTLED = /^\W*(?:decided|not a decision|withdrawn|closed|superseded)\b/i;
/** Finished phases and the index of the work items repeat decisions; they are not the owner of one. */
const NOT_OWNER = /^docs\/(?:plans\/phase-3\/|work-items\/README\.md$)/;
const WAITS = /^(?:Draft|Direction decided|Not checked)\b/;
const LIMIT = 160;
const OLDER = "Checks that other documents hold";

function cells(row: string): string[] {
  return row.split("|").slice(1, -1).map((cell) => cell.trim());
}

function plain(text: string): string {
  return text.replace(LINK, "$1").replace(/\*\*/g, "");
}

function short(text: string): string {
  const stripped = plain(text);
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

/** A row of a table, with the lower-case cells of the header row of its table. */
interface TableRow {
  row: string[];
  head: string[];
}

function tableRows(text: string): TableRow[] {
  const rows: TableRow[] = [];
  let head: string[] = [];
  let inside = false;
  for (const line of text.split("\n")) {
    if (!line.startsWith("|")) inside = false;
    else if (!inside) {
      inside = true;
      head = cells(line).map((cell) => cell.toLowerCase());
    } else rows.push({ row: cells(line), head });
  }
  return rows;
}

/** If the table asks the operator: a recorded decision has the columns `ID | Decision | Reason`. */
function asks(head: string[]): boolean {
  return head.includes("question") || head.includes("recommendation");
}

/** The rows of a decision table whose result does not say that it is settled, with the recommendation. */
function decisions(documents: Document[]): string[] {
  const seen = new Set<string>();
  const rows: string[] = [];
  for (const document of documents.filter((each) => !NOT_OWNER.test(each.path))) {
    for (const { row, head } of tableRows(document.text)) {
      const id = plain(row[0] ?? "");
      const key = GLOBAL.test(id) ? id : `${document.path} ${id}`;
      // A row with two cells is a list of names, not a decision with its result.
      if (!DECISION.test(id) || row.length < 3 || (LOCAL.test(id) && !asks(head)) || seen.has(key) || SETTLED.test(row.at(-1) ?? "")) continue;
      seen.add(key);
      rows.push(`| ${id} | ${short(row[1] ?? "")} | ${short(row.at(-1) ?? "")} | ${linked(document.path)} |`);
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

/** A row of an older check is open when its last cell starts with "Open". */
function isOpen(row: string[]): boolean {
  return row.length > 1 && /^Open\b/.test(row.at(-1) ?? "");
}

/** The open entries (`### HT-nnn`), then the open rows of the tables of older checks. */
function humanChecks(documents: Document[]): string[] {
  const text = documents.find((document) => document.path === CHECKS)?.text ?? "";
  const rows: string[] = [];
  let section = "";
  for (const line of text.split("\n")) {
    if (line.startsWith("## ")) section = line.slice(3).trim();
    const entry = /^### (HT-\d+)[:.] (.+)$/.exec(line);
    if (entry !== null && section === "Open") rows.push(`| ${entry[1] ?? ""} | ${short(entry[2] ?? "")} |`);
    const row = line.startsWith("|") ? cells(line) : [];
    if (section === OLDER && isOpen(row)) rows.push(`| ${plain(row[0] ?? "")} | ${short(row[1] ?? "")} |`);
  }
  return rows;
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
    .sort((a, b) => (a.path < b.path ? -1 : 1));
  return [
    START,
    "",
    ...table("1. In the order of the priorities", "Row | Work | Waits for", ordered(documents)),
    ...table("2. Open decisions", "Decision | Question | Recommendation or state | Document", decisions(documents)),
    ...table("3. Documents that wait for an approval", "Document | Kind | State", approvals(documents)),
    ...table("4. Checks that only the operator can do", "Check | What", humanChecks(documents)),
    END,
  ].join("\n");
}

/** `queue` with its generated part replaced; undefined when it has no markers. */
export function withQueue(queue: string, rows: string): string | undefined {
  return replaceBlock(queue, START, END, rows);
}

/** A state that permits a build (PLN-007). A document with no header, such as a work item, has no state to refuse. */
const BUILDABLE = /^(?:Approved|In progress|Direction decided)\b/;

/** Each `Agent:` clause of a cell: the label starts the cell or follows a sentence, and the clause ends at `Operator:` or the next label. */
const AGENT = /(?<=^|\. |; )Agent: ([^]*?)(?=Operator:|(?<=\. |; )Agent:|$)/g;
const NOTHING = /^(?:nothing|none)\b/i;

function clauseViolations(documents: Document[], row: string[], clause: string): Violation[] {
  const where = `row ${row[0] ?? ""}`;
  const targets = [...clause.matchAll(LINK)].map((match) => posix.normalize(posix.join(posix.dirname(PRIORITIES), (match[2] ?? "").split("#")[0] ?? "")));
  if (targets.length === 0) return [{ path: PRIORITIES, signal: "agent-task-without-link", detail: where }];
  const violations: Violation[] = [];
  for (const target of targets) {
    const state = header(documents.find((document) => document.path === target)?.text ?? "").get("state");
    if (state !== undefined && !BUILDABLE.test(state)) {
      violations.push({ path: PRIORITIES, signal: "agent-task-in-unapproved-document", detail: `${where}: ${target} is ${state}` });
    }
  }
  return violations;
}

/**
 * The tasks for an agent in the priority order: each `Agent:` clause has a link, and each linked
 * document that has a state has a state that permits a build (PLN-007).
 */
function agentTasks(documents: Document[]): Violation[] {
  return priorityRows(documents).flatMap((row) =>
    [...(row.at(-1) ?? "").matchAll(AGENT)]
      .map((match) => match[1] ?? "")
      .filter((clause) => !NOTHING.test(clause))
      .flatMap((clause) => clauseViolations(documents, row, clause)),
  );
}

/** PLN-023: the queue agrees with the documents, and a task for an agent is in a plan. */
export function checkQueue(documents: Document[]): Violation[] {
  const queue = documents.find((document) => document.path === QUEUE);
  const current = queue === undefined ? undefined : withQueue(queue.text, queueRows(documents));
  const violations = agentTasks(documents);
  if (queue !== undefined && current === undefined) {
    violations.push({ path: QUEUE, signal: "queue-without-list", detail: "the two marker comments of the generated part are missing" });
  } else if (queue !== undefined && current !== queue.text) {
    violations.push({ path: QUEUE, signal: "queue-out-of-date", detail: "run `bun run quality:register`" });
  }
  return violations;
}
