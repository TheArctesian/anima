/**
 * Numbers that a merge assigns (PLN-021): a branch writes a placeholder, for example `R-NEW-socket`
 * or `HT-NEW-socket`, and the merge into `dev` gives each placeholder the next free number of its
 * series. Two branches then never take the same number. Pure: files in, files out.
 */

/** One file of the repository as the assignment reads it. */
export interface TextFile {
  path: string;
  text: string;
}

/** A series of identifiers: what stands before the number, and how many digits a number has at least. */
interface Series {
  prefix: string;
  digits: number;
}

/** The series whose numbers a merge assigns. The series of one plan (`OD-ROL-`, `HM-D`) are not here: one plan, one session. */
export const SERIES: readonly Series[] = [
  { prefix: "R-", digits: 1 },
  { prefix: "HT-", digits: 3 },
  { prefix: "SF-", digits: 3 },
  { prefix: "DEF-", digits: 3 },
  { prefix: "OD-W", digits: 1 },
];

const NAME = "[a-z0-9]+";

/** `R-NEW-socket` for the series `R-`; `OD-W-NEW-linux` for the series `OD-W`. */
function placeholderPattern(series: Series): RegExp {
  const lead = series.prefix.endsWith("-") ? series.prefix : `${series.prefix}-`;
  return new RegExp(`(?<![A-Za-z0-9-])${escape(lead)}NEW-${NAME}(?![a-z0-9])`, "g");
}

function numberPattern(series: Series): RegExp {
  return new RegExp(`(?<![A-Za-z0-9-])${escape(series.prefix)}(\\d+)(?![A-Za-z0-9])`, "g");
}

function escape(text: string): string {
  return text.replace(/[.*+?^${}()|[\]\\-]/g, "\\$&");
}

function seriesOf(placeholder: string): Series | undefined {
  return SERIES.find((series) => placeholder.search(placeholderPattern(series)) === 0);
}

/** Each placeholder in the texts and the paths of `files`, one time, in the order of the series and then of the name. */
export function placeholders(files: TextFile[]): string[] {
  const found = new Set<string>();
  for (const series of SERIES) {
    for (const file of files) {
      for (const match of `${file.path}\n${file.text}`.matchAll(placeholderPattern(series))) found.add(match[0]);
    }
  }
  return [...found].sort((one, other) => order(one) - order(other) || one.localeCompare(other));
}

function order(placeholder: string): number {
  return SERIES.findIndex((series) => series === seriesOf(placeholder));
}

/** The highest number of `series` that a text or a path of `files` names; 0 when there is none. */
function highest(files: TextFile[], series: Series): number {
  let most = 0;
  for (const file of files) {
    for (const match of `${file.path}\n${file.text}`.matchAll(numberPattern(series))) {
      most = Math.max(most, Number(match[1]));
    }
  }
  return most;
}

/** What an assignment did: the number of each placeholder, the files with a new text, and the files with a new path. */
export interface Assignment {
  numbers: Map<string, string>;
  changed: TextFile[];
  renamed: { from: string; to: string }[];
}

/**
 * Gives each placeholder of `files` the next free number of its series, in the order of
 * `placeholders`, and puts the number in each text and each path that names the placeholder.
 */
export function assign(files: TextFile[]): Assignment {
  const numbers = new Map<string, string>();
  const next = new Map<Series, number>();
  for (const placeholder of placeholders(files)) {
    const series = seriesOf(placeholder);
    if (series === undefined) continue;
    const number = (next.get(series) ?? highest(files, series)) + 1;
    next.set(series, number);
    numbers.set(placeholder, `${series.prefix}${String(number).padStart(series.digits, "0")}`);
  }
  // The longest first: `R-NEW-ab` must not be read as `R-NEW-a` and a letter.
  const inOrder = [...numbers].sort(([one], [other]) => other.length - one.length);
  const replaced = (text: string) => inOrder.reduce((now, [from, to]) => now.replaceAll(from, to), text);
  const changed: TextFile[] = [];
  const renamed: { from: string; to: string }[] = [];
  for (const file of files) {
    const path = replaced(file.path);
    const text = replaced(file.text);
    if (path !== file.path) renamed.push({ from: file.path, to: path });
    if (text !== file.text) changed.push({ path, text });
  }
  return { numbers, changed, renamed };
}

/** Whether `branch` is one that each session shares: a placeholder must not be there. */
export function isShared(branch: string): boolean {
  return branch === "dev" || branch === "main";
}
