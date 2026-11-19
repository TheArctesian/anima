import type { LogLine } from '../api';
import { lineTime } from '../rows';

// HOST-015: the search, the filter by level, and the text of a download. All of it is work on the lines that the hub
// gave (HOST-001): no hub function, and nothing is kept.

export type Level = 'error' | 'warning' | 'info' | 'debug';

/** The levels the operator can choose; "all" shows each line, also one with no level. */
export const LEVEL_CHOICES = ['all', 'error', 'warning', 'info', 'debug'] as const;
export type LevelChoice = (typeof LEVEL_CHOICES)[number];
export const LEVEL_LABELS: Readonly<Record<LevelChoice, string>> = { all: 'All', error: 'Error', warning: 'Warning', info: 'Info', debug: 'Debug' };

/** A level word as programs write it: `ERROR`, `[warn]`, `level=info`, `"level":"debug"`. The first in the line counts. */
const LEVEL_WORD = /\b(fatal|panic|critical|crit|error|err|warning|warn|info|notice|debug|trace)\b/i;

const LEVELS: Readonly<Record<string, Level>> = {
  fatal: 'error',
  panic: 'error',
  critical: 'error',
  crit: 'error',
  error: 'error',
  err: 'error',
  warning: 'warning',
  warn: 'warning',
  info: 'info',
  notice: 'info',
  debug: 'debug',
  trace: 'debug',
};

/** The level a line states, from its first level word; null when it states none. Only the first 120 characters are read. */
export function levelOf(text: string): Level | null {
  const word = LEVEL_WORD.exec(text.slice(0, 120))?.[1];
  return word === undefined ? null : (LEVELS[word.toLowerCase()] ?? null);
}

export interface LogView {
  /** Text a line must hold, in any case; empty for no search. */
  readonly search: string;
  readonly level: LevelChoice;
}

/** The lines the view shows: those that hold the search text and state the chosen level. */
export function shownLines(lines: readonly LogLine[], view: LogView): readonly LogLine[] {
  const wanted = view.search.trim().toLowerCase();
  return lines.filter((line) => (wanted === '' || line.text.toLowerCase().includes(wanted)) && (view.level === 'all' || levelOf(line.text) === view.level));
}

/** The text of a download: one line for each shown line, with its time as the hub gave it when `timestamps` is on. */
export function downloadText(lines: readonly LogLine[], timestamps: boolean): string {
  return lines.map((line) => (timestamps && line.at != null ? `${line.at} ${line.text}` : line.text)).join('\n') + '\n';
}

/** The name of the file of a download: the name of the container with only safe characters, and `.log`. */
export function downloadName(container: string): string {
  const safe = container.replace(/[^A-Za-z0-9._-]+/g, '-').replace(/^[-.]+|-+$/g, '');
  return `${safe === '' ? 'container' : safe}.log`;
}

export { lineTime };
