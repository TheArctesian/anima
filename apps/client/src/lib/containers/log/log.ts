import type { LogLine } from '../api';
import { lineTime } from '../rows';

// HOST-015: the search, the filter by level, and the text of a download. All of it is work on the lines that the hub
// gave (HOST-001): no hub function, and nothing is kept.

export type Level = 'error' | 'warning' | 'info' | 'debug';

/** The levels the operator can choose; "all" shows each line, also one with no level. */
export const LEVEL_CHOICES = ['all', 'error', 'warning', 'info', 'debug'] as const;
export type LevelChoice = (typeof LEVEL_CHOICES)[number];
export const LEVEL_LABELS: Readonly<Record<LevelChoice, string>> = { all: 'All', error: 'Error', warning: 'Warning', info: 'Info', debug: 'Debug' };

const LEVELS: Readonly<Record<string, Level>> = {
  fatal: 'error',
  fata: 'error',
  panic: 'error',
  pani: 'error',
  critical: 'error',
  crit: 'error',
  severe: 'error',
  emerg: 'error',
  alert: 'error',
  error: 'error',
  erro: 'error',
  err: 'error',
  e: 'error',
  f: 'error',
  warning: 'warning',
  warn: 'warning',
  wrn: 'warning',
  w: 'warning',
  info: 'info',
  inf: 'info',
  notice: 'info',
  i: 'info',
  debug: 'debug',
  debu: 'debug',
  dbg: 'debug',
  trace: 'debug',
  trac: 'debug',
};

const WORDS = 'fatal|fata|panic|pani|critical|crit|severe|emerg|alert|error|erro|err|warning|warn|wrn|info|inf|notice|debug|debu|dbg|trace|trac';
/** The colour codes of a terminal, which some programs write around the level word. */
// eslint-disable-next-line no-control-regex -- the escape character is what a colour code starts with
const COLOUR = /\u001b\[[0-9;]*m/g;
/** A field that names the level: `level=warn`, `"level":"error"`, `lvl=info`, `severity: ERROR`. It wins over every other word. */
const FIELD = new RegExp(`\\b(?:level|lvl|severity)"?\\s*[=:]\\s*"?(${WORDS})\\b`, 'i');
/** A level word in capitals (`ERROR`, `WARN[0001]`), or in brackets or before a colon in any case (`[warn]`, `error:`). */
const WORD = new RegExp(`\\b(${WORDS.toUpperCase()})\\b|\\[(${WORDS})\\]|(?:^|\\s)(${WORDS}):`, '');
const BRACKET_OR_COLON = new RegExp(`\\[(${WORDS})\\]|(?:^|\\s)(${WORDS}):`, 'i');
/** The form of klog: one letter, the month and the day, then the time: `E1118 10:00:00`. */
const KLOG = /^([EWIF])\d{4} \d{2}:\d{2}:\d{2}/;
/** How far into a line a level word is read as its level and not as text of the message. */
const READ_TO = 120;

function word(found: RegExpExecArray | null): Level | null {
  if (found === null || found.index >= READ_TO) return null;
  const text = found.slice(1).find((each) => each !== undefined);
  return text === undefined ? null : (LEVELS[text.toLowerCase()] ?? null);
}

/**
 * The level a line states; null when it states none. A field that names the level wins; then the first level word in
 * capitals, in brackets or before a colon. A word in small letters in the text (`GET /api/info`, `error.log`) is not a
 * level. Programs write levels in many forms, and this reads the common ones.
 */
export function levelOf(text: string): Level | null {
  const plain = text.replace(COLOUR, '');
  const klog = KLOG.exec(plain)?.[1];
  if (klog !== undefined) return LEVELS[klog.toLowerCase()] ?? null;
  const first = [WORD.exec(plain), BRACKET_OR_COLON.exec(plain)].filter((found) => found !== null).sort((a, b) => a.index - b.index)[0] ?? null;
  return word(FIELD.exec(plain)) ?? word(first);
}

export interface LogView {
  /** Text a line must hold, in any case; empty for no search. */
  readonly search: string;
  readonly level: LevelChoice;
}

/**
 * The lines the view shows: those that hold the search text and have the chosen level. A line that states no level has
 * the level of the line before it, so the lines of a stack trace stay with their error.
 */
export function shownLines(lines: readonly LogLine[], view: LogView): readonly LogLine[] {
  const wanted = view.search.trim().toLowerCase();
  let level: Level | null = null;
  return lines.filter((line) => {
    level = levelOf(line.text) ?? level;
    return (wanted === '' || line.text.toLowerCase().includes(wanted)) && (view.level === 'all' || level === view.level);
  });
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
