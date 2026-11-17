import { posix } from "node:path";
import type { Violation } from "../report.ts";
import type { Document } from "./documents.ts";
import { header } from "./register.ts";

/** A claim: one file that says which machine and which coding agent builds a task now (PLN-022). */
export const CLAIM = /^docs\/claims\/(?!README\.md$)[^/]+\.md$/;
const KEYS = ["task", "plan", "machine", "agent", "branch", "started"] as const;
const DAY = /^\d{4}-\d{2}-\d{2}$/;

function escaped(text: string): string {
  return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function fieldViolations(claim: Document, fields: Map<string, string>, exists: (path: string) => boolean): Violation[] {
  const missing = KEYS.filter((key) => (fields.get(key) ?? "") === "");
  const details = missing.map((key) => `no ${key}`);
  const plan = fields.get("plan") ?? "";
  if (plan !== "" && (!plan.endsWith(".md") || !exists(plan))) details.push(`the plan ${plan} does not exist`);
  const started = fields.get("started") ?? "";
  if (started !== "" && !DAY.test(started)) details.push(`started is not a date: ${started}`);
  return details.map((detail) => ({ path: claim.path, signal: "claim-not-complete", detail }));
}

/** PLN-022: each claim names its task, plan, machine, agent, branch and day, and one claim holds a task, whichever plan it names. */
export function checkClaims(documents: Document[], exists: (path: string) => boolean): Violation[] {
  const holder = new Map<string, string>();
  const violations: Violation[] = [];
  for (const claim of documents.filter((document) => CLAIM.test(document.path))) {
    const fields = header(claim.text);
    violations.push(...fieldViolations(claim, fields, exists));
    const task = (fields.get("task") ?? "").trim().toLowerCase();
    const plan = documents.find((document) => document.path === posix.normalize(fields.get("plan") ?? ""));
    // The plan names the task as a word of its own: `T1` is not in a plan that has only `T10`.
    const named = new RegExp(`(?<![A-Za-z0-9-])${escaped(fields.get("task") ?? "")}(?![A-Za-z0-9-])`, "i");
    if (plan !== undefined && !named.test(plan.text)) {
      violations.push({ path: claim.path, signal: "claim-not-complete", detail: `the plan does not name the task ${fields.get("task") ?? ""}` });
    }
    const first = holder.get(task);
    if (first === undefined) holder.set(task, claim.path);
    else violations.push({ path: claim.path, signal: "task-claimed-twice", detail: `${first} holds the same task` });
  }
  return violations;
}
