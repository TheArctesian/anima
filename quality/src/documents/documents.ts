import { posix } from "node:path";
import type { Violation } from "../report.ts";

/** One Markdown file of the repository. */
export interface Document {
  path: string;
  text: string;
}

/** A list that must name each document of a kind (PLN-001, PLN-012). */
export interface Listing {
  index: string;
  member: RegExp;
  signal: string;
  needsStatus: boolean;
}

const PLAN =
  /^docs\/(?!work-plan\.md$)(?:[a-z-]+-plan|vpn-build|design-spec)\.md$|^docs\/programme\/phase-\d-(?:plan|agent-delta)\.md$|^docs\/programme\/proposals\/[^/]+\.md$|^docs\/research\/one-click\/[^/]+-(?:tasks|spec-delta)\.md$/;

export const LISTINGS: Listing[] = [
  { index: "docs/plan/register.md", member: PLAN, signal: "plan-not-in-register", needsStatus: true },
  {
    index: "docs/work-items/README.md",
    member: /^docs\/work-items\/[^/]+\/[^/]+\.md$/,
    signal: "work-item-not-in-index",
    needsStatus: false,
  },
];

const CHECKED = /^(?:docs|contracts)\/.*\.md$|^[^/]+\.md$/;
const LINK = /\]\(([^)\s]+)\)/g;
const DEFINITION = /^\s*(?:[-*]\s+)?\*\*([A-Z][A-Z0-9]*(?:-[A-Z])?-\d{1,3}[a-z]?)\.\*\*/;
const HEADER_LINES = 20;
/** A plan gives its status, or points to the register that holds it (PLN-005). */
const STATUS = /status|plan register/i;

/** The repository paths that the relative links of a document point to. */
export function linkTargets(document: Document): string[] {
  const targets: string[] = [];
  for (const match of document.text.matchAll(LINK)) {
    const target = (match[1] ?? "").split("#")[0] ?? "";
    if (target === "" || /^[a-z][a-z0-9+.-]*:/i.test(target)) continue;
    const joined = posix.normalize(posix.join(posix.dirname(document.path), target));
    targets.push(joined.replace(/\/$/, ""));
  }
  return targets;
}

/** The requirement identifiers that a document defines: a bold identifier that starts a line. */
export function definedIdentifiers(text: string): string[] {
  const found: string[] = [];
  for (const line of text.split("\n")) {
    const match = DEFINITION.exec(line);
    if (match?.[1] !== undefined) found.push(match[1]);
  }
  return found;
}

function brokenLinks(documents: Document[], exists: (path: string) => boolean): Violation[] {
  return documents.flatMap((document) =>
    linkTargets(document)
      .filter((target) => target.startsWith("..") || !exists(target))
      .map((target) => ({ path: document.path, signal: "broken-link", detail: target })),
  );
}

function unlisted(documents: Document[], listing: Listing): Violation[] {
  const index = documents.find((document) => document.path === listing.index);
  const named = new Set(index === undefined ? [] : linkTargets(index));
  const members = documents.filter((document) => listing.member.test(document.path));
  const missing = members
    .filter((document) => !named.has(document.path))
    .map((document) => ({ path: document.path, signal: listing.signal, detail: `no link in ${listing.index}` }));
  const silent = members
    .filter((document) => listing.needsStatus && !STATUS.test(document.text.split("\n", HEADER_LINES).join("\n")))
    .map((document) => ({ path: document.path, signal: "plan-without-status", detail: `no status and no pointer to the plan register in its first ${HEADER_LINES} lines` }));
  return [...missing, ...silent];
}

function repeatedDefinitions(documents: Document[], known: ReadonlySet<string>): Violation[] {
  const owner = new Map<string, string>();
  const violations: Violation[] = [];
  for (const document of documents) {
    for (const identifier of new Set(definedIdentifiers(document.text))) {
      const first = owner.get(identifier);
      if (first === undefined) owner.set(identifier, document.path);
      else if (!known.has(identifier)) {
        violations.push({ path: document.path, signal: "identifier-defined-twice", detail: `${identifier} is also in ${first}` });
      }
    }
  }
  return violations;
}

/** The identifiers of the known-duplicates file: one for each line, `#` starts a comment. */
export function parseKnown(text: string): Set<string> {
  const lines = text.split("\n").map((line) => (line.split("#")[0] ?? "").trim());
  return new Set(lines.filter((line) => line !== ""));
}

/**
 * PLN-017: the links resolve, each plan and each work item is in its list, a plan
 * shows its status, and one document defines each requirement identifier.
 */
export function checkDocuments(all: Document[], exists: (path: string) => boolean, known: ReadonlySet<string>): Violation[] {
  const documents = all.filter((document) => CHECKED.test(document.path)).sort((a, b) => a.path.localeCompare(b.path));
  return [
    ...brokenLinks(documents, exists),
    ...LISTINGS.flatMap((listing) => unlisted(documents, listing)),
    ...repeatedDefinitions(documents, known),
  ];
}
