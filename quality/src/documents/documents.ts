import { posix } from "node:path";
import type { Violation } from "../report.ts";
import { checkClaims } from "./claims.ts";
import { checkMockups } from "./mockups.ts";
import { checkQueue } from "./queue.ts";
import { checkRegister } from "./register.ts";
import { repeatedRulings } from "./rulings.ts";

/** One Markdown file of the repository. */
export interface Document {
  path: string;
  text: string;
}

/** A list that must name each document of a kind (PLN-001, PLN-012, PLN-019). */
export interface Listing {
  index: string;
  member: RegExp;
  signal: string;
}

/** A specification, a proposal, a plan or a task breakdown: the folder gives the stage (PLN-018). */
const PLAN =
  /^docs\/spec\/[^/]+\.md$|^docs\/proposals\/[^/]+\.md$|^docs\/plans\/(?:[^/]+\/)?(?:[^/]*-(?:plan|tasks|spec-delta|agent-delta|direction)|vpn-build)\.md$/;

/** A mockup: a static HTML file of a layout (PLN-019). Its text is not read. */
export const MOCKUP = /^docs\/mockup\/.+\.html$/;

export const LISTINGS: Listing[] = [
  { index: "docs/plan/register.md", member: PLAN, signal: "plan-not-in-register" },
  {
    index: "docs/work-items/README.md",
    member: /^docs\/work-items\/[^/]+\/[^/]+\.md$/,
    signal: "work-item-not-in-index",
  },
  { index: "docs/mockup/README.md", member: MOCKUP, signal: "mockup-not-in-index" },
];

const CHECKED = /^(?:docs|contracts|deploy)\/.*\.md$|^[^/]+\.md$|^docs\/mockup\/.+\.html$/;
const LINK = /\]\(([^)\s]+)\)/g;
/** `**DEV-001.**` or `**DEV-001. A title.**`, at the start of a line, a list item or a numbered item. */
const DEFINITION = /^\s*(?:[-*]\s+|\d+\.\s+)?\*\*([A-Z][A-Z0-9]+(?:-[A-Z])?-\d{1,3}[a-z]?)\.(?:\*\*|\s)/;

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
  return documents
    .filter((document) => listing.member.test(document.path) && !named.has(document.path))
    .map((document) => ({ path: document.path, signal: listing.signal, detail: `no link in ${listing.index}` }));
}

const KNOWN = "quality/duplicate-identifiers.txt";

function repeatedDefinitions(documents: Document[], known: ReadonlySet<string>): Violation[] {
  const owner = new Map<string, string>();
  const repeated = new Set<string>();
  const violations: Violation[] = [];
  // The archive is a record: a number of a finished package is not a requirement of today.
  for (const document of documents.filter((each) => !each.path.startsWith("docs/archive/"))) {
    for (const identifier of new Set(definedIdentifiers(document.text))) {
      const first = owner.get(identifier);
      if (first === undefined) owner.set(identifier, document.path);
      else {
        repeated.add(identifier);
        if (!known.has(identifier)) violations.push({ path: document.path, signal: "identifier-defined-twice", detail: `${identifier} is also in ${first}` });
      }
    }
  }
  // The list can only get shorter: an identifier that has one owner now must leave it.
  const single = [...known].filter((identifier) => !repeated.has(identifier));
  return [...violations, ...single.map((identifier) => ({ path: KNOWN, signal: "known-duplicate-has-one-owner", detail: identifier }))];
}

/** The identifiers of the known-duplicates file: one for each line, `#` starts a comment. */
export function parseKnown(text: string): Set<string> {
  const lines = text.split("\n").map((line) => (line.split("#")[0] ?? "").trim());
  return new Set(lines.filter((line) => line !== ""));
}

/**
 * PLN-017: the links resolve, each plan, each work item and each mockup is in its list, a plan
 * gives its kind and its state in its header, the register agrees with the headers (PLN-020),
 * a number of the rulings log names one ruling (PLN-021), a claim is complete and one claim
 * holds a task (PLN-022), no screen changed after its mockup was compared (PLN-019), and one
 * document defines each requirement identifier.
 */
export function checkDocuments(
  all: Document[],
  exists: (path: string) => boolean,
  known: ReadonlySet<string>,
  changed: (screen: string, commit: string) => number | undefined = () => 0,
): Violation[] {
  const documents = all.filter((document) => CHECKED.test(document.path)).sort((a, b) => (a.path < b.path ? -1 : 1));
  return [
    ...brokenLinks(documents, exists),
    ...LISTINGS.flatMap((listing) => unlisted(documents, listing)),
    ...checkRegister(documents, (path) => PLAN.test(path)),
    ...repeatedRulings(documents),
    ...checkClaims(documents, exists),
    ...checkMockups(documents, changed),
    ...checkQueue(documents),
    ...repeatedDefinitions(documents, known),
  ];
}
