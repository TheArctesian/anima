import { latestOf, type ChartPoint } from './plot';

/** A series as the chart's text alternative reads it. */
interface LabelledSeries {
  readonly label: string;
  readonly points: readonly ChartPoint[];
}

/** What the chart's text alternative says. */
interface ChartText {
  readonly title: string;
  readonly series: readonly LabelledSeries[];
  /** The value at the top of the plot. */
  readonly top: number;
  readonly format: (value: number) => string;
  /** Said instead when no series has a value; "<title>: no values yet" by default. */
  readonly empty?: string;
}

/**
 * The chart's `aria-label`: the title, the latest value of each series and the top of the value axis. With more than
 * one series each latest value is named by its series, so the text never relies on colour.
 */
export function chartLabel({ title, series, top, format, empty }: ChartText): string {
  const latest = series.map((one) => ({ label: one.label, value: latestOf(one.points) }));
  if (latest.every((one) => one.value === null)) return empty ?? `${title}: no values yet`;
  const said = (value: number | null) => (value === null ? 'no value' : format(value));
  const values =
    latest.length === 1
      ? `latest ${said(latest[0]?.value ?? null)}`
      : latest.map((one) => `${one.label} latest ${said(one.value)}`).join(', ');
  return `${title}, ${values}, top of the chart ${format(top)}`;
}
