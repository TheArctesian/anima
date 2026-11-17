import { posix } from "node:path";
import type { Violation } from "../report.ts";
import type { Document } from "./documents.ts";

/** The register: the file that holds the generated list of the plans (PLN-001, PLN-020). */
export const REGISTER = "docs/plan/register.md";
/** Finished work. It is a record: it has no row in the register. */
const ARCHIVE = "docs/archive/";
const START = "<!-- generated from the header of each document: bun run quality:register -->";
const END = "<!-- end of the generated list -->";
const LINK = /\]\(([^)\s]+)\)/g;
/** The first word or words of a state (PLN-007). */
const STATE = /^(?:Draft|Direction decided|Approved|In progress|Done|Parked|Withdrawn|Not checked)\b/;
const KINDS = ["Specification", "Plan", "Build specification", "Task breakdown", "Proposal"];

/** The `key: value` lines between the two `---` lines that start a document. */
export function header(text: string): Map<string, string> {
  const fields = new Map<string, string>();
  const lines = text.split("\n");
  if (lines[0] !== "---") return fields;
  for (const line of lines.slice(1)) {
    if (line === "---") break;
    const at = line.indexOf(": ");
    if (at > 0) fields.set(line.slice(0, at), line.slice(at + 2).trim());
  }
  return fields;
}

/** `text` of the document `from`, with each relative link made relative to the register. */
function rebased(text: string, from: string): string {
  return text.replace(LINK, (whole, target: string) => {
    if (/^[a-z][a-z0-9+.-]*:|^#/i.test(target)) return whole;
    const absolute = posix.normalize(posix.join(posix.dirname(from), target));
    return `](${posix.relative(posix.dirname(REGISTER), absolute)})`;
  });
}

function rank(kind: string): number {
  const found = KINDS.findIndex((known) => kind.startsWith(known));
  return found === -1 ? KINDS.length : found;
}

/** The generated list: one row for each document whose header gives a kind and a state. */
export function registerRows(documents: Document[]): string {
  const rows = documents
    .map((document) => ({ path: document.path, fields: header(document.text) }))
    .filter(({ path, fields }) => path.startsWith("docs/") && !path.startsWith(ARCHIVE) && fields.has("kind") && fields.has("state"))
    .sort((a, b) => rank(a.fields.get("kind") ?? "") - rank(b.fields.get("kind") ?? "") || (a.path < b.path ? -1 : 1))
    .map(({ path, fields }) => {
      const link = posix.relative(posix.dirname(REGISTER), path);
      const evidence = rebased(fields.get("evidence") ?? "", path);
      return `| [${link.replace(/^\.\.\//, "")}](${link}) | ${fields.get("kind") ?? ""} | ${fields.get("state") ?? ""} | ${evidence} |`;
    });
  return [START, "", "| Document | Kind | State | Evidence |", "| --- | --- | --- | --- |", ...rows, "", END].join("\n");
}

/** If `register` has the two markers of its generated list. Without them nothing is checked. */
export function hasList(register: string): boolean {
  return register.includes(START) && register.includes(END);
}

/** `register` with its generated list replaced by `rows`; undefined when it has no markers. */
export function withRows(register: string, rows: string): string | undefined {
  const start = register.indexOf(START);
  const end = register.indexOf(END);
  if (start === -1 || end === -1) return undefined;
  return register.slice(0, start) + rows + register.slice(end + END.length);
}

/** PLN-020: a plan gives its kind and its state in its header, and the register agrees. */
export function checkRegister(documents: Document[], isPlan: (path: string) => boolean): Violation[] {
  const violations: Violation[] = [];
  for (const document of documents.filter((each) => isPlan(each.path))) {
    const fields = header(document.text);
    if (!fields.has("kind") || !STATE.test(fields.get("state") ?? "")) {
      violations.push({ path: document.path, signal: "plan-without-state", detail: "its header needs `kind:` and a `state:` that starts with a word of PLN-007" });
    } else if ((fields.get("evidence") ?? "") === "") {
      violations.push({ path: document.path, signal: "plan-without-evidence", detail: "its header needs `evidence:` (PLN-009)" });
    }
  }
  const register = documents.find((document) => document.path === REGISTER);
  const current = register === undefined ? undefined : withRows(register.text, registerRows(documents));
  if (register !== undefined && current !== undefined && current !== register.text) {
    violations.push({ path: REGISTER, signal: "register-out-of-date", detail: "run `bun run quality:register`" });
  }
  return violations;
}
