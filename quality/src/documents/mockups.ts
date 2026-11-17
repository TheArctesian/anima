import { posix } from "node:path";
import type { Violation } from "../report.ts";
import type { Document } from "./documents.ts";

/** The index of the mockups: one row for each mockup, with its screens and the commit they were compared at (PLN-019). */
export const MOCKUPS = "docs/mockup/README.md";
const LINK = /\]\(([^)\s#]+)\)/g;
const COMMIT = /`([0-9a-f]{7,40})`/;

/** One row of the index: the screens of the application, the state and the commit of the last comparison. */
export interface MockupRow {
  name: string;
  screens: string[];
  built: boolean;
  compared: string | undefined;
}

function targets(cell: string): string[] {
  return [...cell.matchAll(LINK)].map((match) => posix.normalize(posix.join(posix.dirname(MOCKUPS), match[1] ?? "")));
}

/** The rows of the table under "## Index". The columns are: Mockup, Owner, Screens, State, Compared, Differences. */
export function mockupRows(text: string): MockupRow[] {
  const start = text.indexOf("\n## Index");
  if (start < 0) return [];
  const rows = text.slice(start).split("\n").filter((line) => line.startsWith("|")).slice(2);
  return rows.map((row) => {
    const cells = row.split("|").slice(1, -1).map((cell) => cell.trim());
    return {
      name: cells[0] ?? "",
      screens: targets(cells[2] ?? ""),
      built: /^\W*built\b/i.test(cells[3] ?? ""),
      compared: COMMIT.exec(cells[4] ?? "")?.[1],
    };
  });
}

/**
 * PLN-019: a mockup that is Built names the commit it was compared at, and no layout file of
 * its screens changed after that one. `changedSince` gives the number of those files,
 * or undefined when the repository does not have the commit.
 */
export function checkMockups(documents: Document[], changedSince: (screen: string, commit: string) => number | undefined): Violation[] {
  const index = documents.find((document) => document.path === MOCKUPS);
  const violations: Violation[] = [];
  for (const row of mockupRows(index?.text ?? "")) {
    const commit = row.compared;
    if (commit === undefined) {
      if (row.built) violations.push({ path: MOCKUPS, signal: "mockup-not-compared", detail: row.name });
      continue;
    }
    if (row.built && row.screens.length === 0) violations.push({ path: MOCKUPS, signal: "mockup-without-screens", detail: row.name });
    for (const screen of row.screens) {
      const count = changedSince(screen, commit);
      if (count === undefined) violations.push({ path: MOCKUPS, signal: "mockup-commit-not-found", detail: `${commit} of ${row.name}` });
      else if (count > 0) {
        violations.push({ path: MOCKUPS, signal: "screen-changed-after-mockup", detail: `${screen} has ${count} layout file(s) that changed after ${commit}; compare ${row.name} again` });
      }
    }
  }
  return violations;
}
