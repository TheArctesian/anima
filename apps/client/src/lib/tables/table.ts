/** What a severity cell shows: a word and a dot in the severity's colour (UI-003). */
export type Severity = 'info' | 'warning' | 'error';

export const SEVERITY_WORDS: Record<Severity, string> = { info: 'Info', warning: 'Warning', error: 'Error' };

/** How a column is drawn. `number` is right-aligned in tabular figures (UI-025); `custom` is drawn by the table's `cell` snippet. */
export type ColumnKind = 'text' | 'number' | 'severity' | 'custom';

export interface Column<Row> {
  readonly key: string;
  readonly header: string;
  readonly kind?: ColumnKind;
  /** The cell's text for `text` and `number` columns. */
  readonly value?: (row: Row) => string;
  /** The row's severity for a `severity` column. */
  readonly severity?: (row: Row) => Severity;
  /** The narrowest the column may be, in rem. The table never goes narrower than the sum of these. */
  readonly min: number;
  /** The widest the column grows, in rem. A column without `max` and with `flex` takes what is left. */
  readonly max?: number;
  readonly flex?: boolean;
}

/** The width a `<col>` is given: a flexible column takes what is left, any other its `max` (or `min`). */
export function columnWidth<Row>(column: Column<Row>): string | undefined {
  if (column.flex === true) return undefined;
  return `${String(column.max ?? column.min)}rem`;
}

/** The narrowest the whole table may be: the sum of the columns' minimums, in rem. */
export function tableMinWidth<Row>(columns: readonly Column<Row>[]): string {
  return `${String(columns.reduce((sum, column) => sum + column.min, 0))}rem`;
}

/** A row of the body: a group header, or a data row. */
export type BodyEntry<Row> = { readonly group: string } | { readonly row: Row };

/** The rows in order, with a group header wherever `groupOf` gives a new label; without `groupOf` there are none. */
export function withGroupHeaders<Row>(rows: readonly Row[], groupOf?: (row: Row) => string): readonly BodyEntry<Row>[] {
  if (groupOf === undefined) return rows.map((row) => ({ row }));
  const entries: BodyEntry<Row>[] = [];
  let current: string | undefined;
  for (const row of rows) {
    const group = groupOf(row);
    if (group !== current) entries.push({ group });
    current = group;
    entries.push({ row });
  }
  return entries;
}

function startOfDay(moment: Date): number {
  return new Date(moment.getFullYear(), moment.getMonth(), moment.getDate()).getTime();
}

const DAY_MS = 86_400_000;
const DATE = new Intl.DateTimeFormat('en-GB', { day: 'numeric', month: 'short', year: 'numeric' });

/** The group header for a row's time: "Today", "Yesterday", then the date. Calendar days in the local time zone. */
export function dayLabel(at: Date, now: Date): string {
  const days = Math.round((startOfDay(now) - startOfDay(at)) / DAY_MS);
  if (days === 0) return 'Today';
  return days === 1 ? 'Yesterday' : DATE.format(at);
}
