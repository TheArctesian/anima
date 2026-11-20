/**
 * The record of each mockup (PLN-019): one file for each mockup in `docs/mockup/records/`, with
 * the same six fields as a row of the index. The index table of `docs/mockup/README.md` is
 * written from the records by `bun run quality:register`, so two branches that compare different
 * mockups change different files and do not conflict. Pure.
 */
import { posix } from "node:path";
import type { Document } from "./documents.ts";
import { replaceBlock } from "./generated.ts";

export const MOCKUPS = "docs/mockup/README.md";
export const RECORDS = "docs/mockup/records/";
export const FIELDS = ["Mockup", "Owner", "Screens", "State", "Compared", "Differences"] as const;
export type Field = (typeof FIELDS)[number];
export type Cells = Record<Field, string>;

const START = "<!-- generated from docs/mockup/records/: bun run quality:register -->";
const END = "<!-- end of the generated table -->";
const LINK = /\]\(([^)\s]+)\)/g;
const FIELD = /^- (Mockup|Owner|Screens|State|Compared|Differences): ?(.*)$/;

/** Whether `path` is the record of a mockup. */
export function isRecord(path: string): boolean {
  return path.startsWith(RECORDS) && path.endsWith(".md") && !path.slice(RECORDS.length).includes("/");
}

/** `text` with each relative link, written from the folder `from`, written from the folder `to`. */
function rebased(text: string, from: string, to: string): string {
  return text.replace(LINK, (whole, target: string) => {
    if (/^[a-z][a-z0-9+.-]*:|^#/i.test(target)) return whole;
    return `](${posix.relative(to, posix.normalize(posix.join(from, target)))})`;
  });
}

/** The record path of a row whose Mockup cell links first to `docs/mockup/<a>/index.html` or `<a>.html`. */
export function recordPath(mockupCell: string): string | undefined {
  const target = /\]\(([^)\s#]+)\)/.exec(mockupCell)?.[1];
  if (target === undefined) return undefined;
  const name = target.replace(/\.html$/, "").replace(/\/index$/, "").replaceAll("/", "-");
  return `${RECORDS}${name}.md`;
}

/** The fields of a record, with its links written from the folder of the index; undefined without a Mockup field. */
export function recordCells(text: string): Cells | undefined {
  const found = new Map<string, string>();
  for (const line of text.split("\n")) {
    const match = FIELD.exec(line);
    if (match?.[1] !== undefined) found.set(match[1], rebased(match[2] ?? "", RECORDS, posix.dirname(MOCKUPS)));
  }
  if (!found.has("Mockup")) return undefined;
  return Object.fromEntries(FIELDS.map((field) => [field, found.get(field) ?? ""])) as Cells;
}

/** The text of a record from the cells of a row of the index (links written from the folder of the index). */
export function recordText(cells: Cells): string {
  const name = /\]\(([^)\s#]+)\)/.exec(cells.Mockup)?.[1] ?? cells.Mockup;
  const lines = FIELDS.map((field) => `- ${field}: ${rebased(cells[field], posix.dirname(MOCKUPS), RECORDS)}`);
  return [`# The record of the mockup ${name}`, "", "One row of the index in [README.md](../README.md) (PLN-019). `bun run quality:register` writes the row from this file.", "", ...lines, ""].join("\n");
}

/** The rows of an index table in the form before the records: the cells of each row of "## Index". */
export function tableRows(text: string): Cells[] {
  const start = text.indexOf("\n## Index");
  if (start < 0) return [];
  const rows = text.slice(start).split("\n").filter((line) => line.startsWith("|")).slice(2);
  return rows.map((row) => {
    const cells = row.split("|").slice(1, -1).map((cell) => cell.trim());
    return Object.fromEntries(FIELDS.map((field, at) => [field, cells[at] ?? ""])) as Cells;
  });
}

/** Whether the index holds a generated table (the form of the records). */
export function isGenerated(index: string): boolean {
  return index.includes(START);
}

/** The generated table: one row for each record, in the order of the record paths. */
export function mockupTable(records: Document[]): string {
  const rows = records
    .filter((record) => isRecord(record.path))
    .sort((a, b) => (a.path < b.path ? -1 : 1))
    .flatMap((record) => {
      const cells = recordCells(record.text);
      return cells === undefined ? [] : [`| ${FIELDS.map((field) => cells[field]).join(" | ")} |`];
    });
  return [START, "", `| ${FIELDS.join(" | ")} |`, `| ${FIELDS.map(() => "---").join(" | ")} |`, ...rows, "", END].join("\n");
}

/** `index` with its generated table replaced; undefined when it has no markers. */
export function withMockupTable(index: string, table: string): string | undefined {
  return replaceBlock(index, START, END, table);
}
