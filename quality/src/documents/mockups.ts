import { posix } from "node:path";
import type { Violation } from "../report.ts";
import type { Document } from "./documents.ts";

/** The index of the mockups: one row for each mockup, with its screens and the day they were compared (PLN-019). */
export const MOCKUPS = "docs/mockup/README.md";
const LINK = /\]\(([^)\s#]+)\)/g;
const DAY = /\b(\d{4}-\d{2}-\d{2})\b/;

/** One row of the index: the screens of the application, the state and the day of the last comparison. */
export interface MockupRow {
  name: string;
  screens: string[];
  state: string;
  compared: string | undefined;
}

function targets(cell: string): string[] {
  return [...cell.matchAll(LINK)].map((match) => posix.normalize(posix.join(posix.dirname(MOCKUPS), match[1] ?? "")));
}

/** The rows of the table under "## Index". The columns are: Mockup, Owner, Screens, State, Compared, Differences. */
export function mockupRows(text: string): MockupRow[] {
  const start = text.indexOf("\n## Index");
  if (start < 0) return [];
  const rows = text
    .slice(start)
    .split("\n")
    .filter((line) => line.startsWith("|"))
    .slice(2);
  return rows.map((row) => {
    const cells = row.slice(1, -1).split(" | ").map((cell) => cell.trim());
    return {
      name: cells[0] ?? "",
      screens: targets(cells[2] ?? ""),
      state: (cells[3] ?? "").split(/[ (]/)[0] ?? "",
      compared: DAY.exec(cells[4] ?? "")?.[1],
    };
  });
}

/**
 * PLN-019: a mockup that is Built gives the day it was compared, and no layout file of its
 * screens changed after that day. `changed` gives the day of the last change of a screen.
 */
export function checkMockups(documents: Document[], changed: (screen: string) => string | undefined): Violation[] {
  const index = documents.find((document) => document.path === MOCKUPS);
  const violations: Violation[] = [];
  for (const row of mockupRows(index?.text ?? "")) {
    if (row.compared === undefined) {
      if (row.state === "Built") violations.push({ path: MOCKUPS, signal: "mockup-not-compared", detail: row.name });
      continue;
    }
    for (const screen of row.screens) {
      const day = changed(screen);
      if (day !== undefined && day > row.compared) {
        violations.push({
          path: MOCKUPS,
          signal: "screen-changed-after-mockup",
          detail: `${screen} changed on ${day}; ${row.name} was compared on ${row.compared}`,
        });
      }
    }
  }
  return violations;
}
