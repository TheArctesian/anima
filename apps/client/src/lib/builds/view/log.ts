import { LineCleaner } from '../redaction';
import type { LogLineView } from './api';

/** `known` with `added` merged in by sequence: a line seen twice (a page and the subscription) counts once, and the order is the job's (BLD-081). */
export function mergeLines(known: readonly LogLineView[], added: readonly LogLineView[]): LogLineView[] {
  const bySequence = new Map(known.map((line) => [line.sequence, line]));
  for (const line of added) bySequence.set(line.sequence, line);
  return [...bySequence.values()].toSorted((a, b) => a.sequence - b.sequence);
}

/** The sequence a next page of the log starts after, or `null` for no line yet. */
export function lastSequence(lines: readonly LogLineView[]): number | null {
  return lines.at(-1)?.sequence ?? null;
}

/**
 * The text to show: every line cleaned again here, so a secret the hub's redaction missed is not drawn (BLD-049, LOG-007).
 * The lines are cleaned in order, so a key block is removed whole.
 */
export function shownLines(lines: readonly LogLineView[]): string[] {
  const cleaner = new LineCleaner(() => []);
  return lines.map((line) => cleaner.clean(line.text));
}

/** A failing test, a compiler error or a panic: the lines an operator looks for in a failed job's log. */
const FAILURE_LINE = /\(fail\)|^\s*error(?:\[\w+\])?:|\bFAIL\b|\bpanicked at\b|\bERR!/;

/** Whether a shown line reports a failure, so the log can mark it. */
export function isFailureLine(text: string): boolean {
  return FAILURE_LINE.test(text);
}
