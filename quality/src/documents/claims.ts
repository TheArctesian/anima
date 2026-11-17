import type { Violation } from "../report.ts";
import type { Document } from "./documents.ts";

/** A claim: one file that says which machine and which coding agent builds a task now (PLN-022). */
export const CLAIM = /^docs\/claims\/(?!README\.md$)[^/]+\.md$/;
const KEYS = ["task", "plan", "machine", "agent", "branch", "started"] as const;
const DAY = /^\d{4}-\d{2}-\d{2}$/;

/** The `key: value` lines between the two `---` lines that start a claim. */
export function claimFields(text: string): Map<string, string> {
  const fields = new Map<string, string>();
  const lines = text.split("\n");
  if (lines[0]?.trim() !== "---") return fields;
  for (const line of lines.slice(1)) {
    if (line.trim() === "---") break;
    const colon = line.indexOf(":");
    if (colon > 0) fields.set(line.slice(0, colon).trim(), line.slice(colon + 1).trim());
  }
  return fields;
}

function fieldViolations(claim: Document, fields: Map<string, string>, exists: (path: string) => boolean): Violation[] {
  const missing = KEYS.filter((key) => (fields.get(key) ?? "") === "");
  const details = missing.map((key) => `no ${key}`);
  const plan = fields.get("plan") ?? "";
  if (plan !== "" && !exists(plan)) details.push(`the plan ${plan} does not exist`);
  const started = fields.get("started") ?? "";
  if (started !== "" && !DAY.test(started)) details.push(`started is not a date: ${started}`);
  return details.map((detail) => ({ path: claim.path, signal: "claim-not-complete", detail }));
}

/** PLN-022: each claim names its task, plan, machine, agent, branch and day, and one claim holds a task. */
export function checkClaims(documents: Document[], exists: (path: string) => boolean): Violation[] {
  const holder = new Map<string, string>();
  const violations: Violation[] = [];
  for (const claim of documents.filter((document) => CLAIM.test(document.path))) {
    const fields = claimFields(claim.text);
    violations.push(...fieldViolations(claim, fields, exists));
    const task = `${fields.get("plan") ?? ""} ${fields.get("task") ?? ""}`;
    const first = holder.get(task);
    if (first === undefined) holder.set(task, claim.path);
    else violations.push({ path: claim.path, signal: "task-claimed-twice", detail: `${first} holds the same task` });
  }
  return violations;
}
