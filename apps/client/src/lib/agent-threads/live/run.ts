/**
 * The frames of the local socket of the thread runner (TH14, THR-128), as the desktop shell passes them: the threads that
 * the daemon of this computer runs, their updates, and the calls that drive them. Each frame crosses a process
 * boundary, so each field is checked; a frame that does not read is left out.
 */

import type { Program } from '../thread';

export type RunState = 'starting' | 'working' | 'needs_you' | 'idle' | 'stopped' | 'unknown';
export type Mode = 'full_access' | 'accept_edits' | 'ask';
export type Effort = 'low' | 'medium' | 'high' | 'xhigh' | 'max';
export type RequestKind = 'command' | 'file_change' | 'tool' | 'question';

/** A thread that the runner started. */
export interface RunRow {
  readonly id: string;
  readonly program: Program;
  readonly folder: string;
  readonly mode: Mode;
  readonly model: string | null;
  readonly state: RunState;
  /** The program's own ID of the session or thread, when it named one. */
  readonly session: string | null;
  readonly title: string;
  readonly started: Date;
  /** Why the program ended, when it did. */
  readonly ended: string | null;
  /** The runner's IDs of the requests that wait for an answer. */
  readonly open: readonly string[];
}

export interface Question {
  readonly key: string;
  readonly header: string | null;
  readonly text: string;
  readonly choices: readonly string[];
  readonly many: boolean;
}

/** What a program asks the person, as the program named it (THR-032). */
export interface Ask {
  readonly kind: RequestKind;
  readonly tool: string;
  readonly subject: string;
  readonly diff: string | null;
  readonly sessionRules: readonly string[];
  readonly questions: readonly Question[];
}

/** Something the program said. Only the kinds that the page draws are kept. */
export type Said =
  | { readonly kind: 'text_delta'; readonly text: string }
  | { readonly kind: 'message'; readonly text: string }
  | { readonly kind: 'tool_call'; readonly id: string; readonly name: string; readonly input: string }
  | { readonly kind: 'tool_output_delta'; readonly id: string; readonly text: string }
  | { readonly kind: 'tool_output'; readonly id: string; readonly output: string; readonly failed: boolean }
  | { readonly kind: 'turn_ended'; readonly outcome: string; readonly detail: string | null }
  | { readonly kind: 'notice'; readonly text: string };

export type Change =
  | { readonly change: 'said'; readonly event: Said }
  | { readonly change: 'asked'; readonly request: string; readonly ask: Ask }
  | { readonly change: 'closed'; readonly request: string; readonly why: string }
  | { readonly change: 'prompt'; readonly text: string; readonly source: string }
  | { readonly change: 'state'; readonly state: RunState; readonly row: RunRow };

export interface Update {
  readonly thread: string;
  readonly seq: number;
  readonly change: Change;
}

export type Refusal =
  | 'unknown_thread'
  | 'folder_not_permitted'
  | 'program_not_found'
  | 'bad_value'
  | 'request_gone'
  | 'answer_does_not_fit'
  | 'not_running'
  | 'holder'
  | 'over_limit';

export interface Limits {
  readonly depth: number;
  readonly children: number;
  readonly running: number;
}

export type Outcome =
  | { readonly outcome: 'ok' }
  | { readonly outcome: 'started'; readonly thread: string }
  | { readonly outcome: 'threads'; readonly threads: readonly RunRow[] }
  | { readonly outcome: 'limits'; readonly limits: Limits }
  | { readonly outcome: 'stopped'; readonly allEnded: boolean }
  | { readonly outcome: 'refused'; readonly why: Refusal };

export type Answer =
  | { readonly answer: 'allow' }
  | { readonly answer: 'allow_for_session'; readonly rule: number }
  | { readonly answer: 'deny'; readonly message: string }
  | { readonly answer: 'choose'; readonly answers: readonly (readonly [string, string])[] };

/** A call to the runner, in the form of its socket. */
export type Action =
  | { readonly call: 'list' }
  | {
      readonly call: 'start';
      readonly program: Program;
      readonly folder: string;
      readonly mode: Mode;
      readonly model: string | null;
      readonly effort: Effort | null;
      readonly prompt: string;
    }
  | { readonly call: 'send'; readonly thread: string; readonly text: string }
  | { readonly call: 'answer'; readonly thread: string; readonly request: string; readonly answer: Answer }
  | { readonly call: 'interrupt'; readonly thread: string }
  | { readonly call: 'stop'; readonly thread: string }
  | { readonly call: 'raise_limit' };

/**
 * The sentence of the operator at the soft limit (THR-114), the same words as `anima agents start` and the runner.
 * Kept word for word.
 */
export const OVER_LIMIT = 'You are over the recommended guidance and at risk of cyberpsychosis from running this many sessions. Do you want to raise the limit?';

const PROGRAMS: readonly string[] = ['claude_code', 'codex'];
const STATES: readonly string[] = ['starting', 'working', 'needs_you', 'idle', 'stopped', 'unknown'];
const MODES: readonly string[] = ['full_access', 'accept_edits', 'ask'];
const KINDS: readonly string[] = ['command', 'file_change', 'tool', 'question'];
const REFUSALS: readonly string[] = ['unknown_thread', 'folder_not_permitted', 'program_not_found', 'bad_value', 'request_gone', 'answer_does_not_fit', 'not_running', 'holder', 'over_limit'];

type Fields = Record<string, unknown>;

function isRecord(value: unknown): value is Fields {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function str(value: unknown): string | null {
  return typeof value === 'string' ? value : null;
}

function oneOf<T extends string>(value: unknown, list: readonly string[]): T | null {
  return typeof value === 'string' && list.includes(value) ? (value as T) : null;
}

function strings(value: unknown): readonly string[] | null {
  return Array.isArray(value) && value.every((each) => typeof each === 'string') ? (value as string[]) : null;
}

/** A row of the runner, or `null` when it does not read. */
export function runRowFrom(value: unknown): RunRow | null {
  if (!isRecord(value)) return null;
  const id = str(value.id);
  const program = oneOf<Program>(value.program, PROGRAMS);
  const folder = str(value.folder);
  const mode = oneOf<Mode>(value.mode, MODES);
  const state = oneOf<RunState>(value.state, STATES);
  const title = str(value.title);
  const started = typeof value.started === 'string' ? new Date(value.started) : null;
  const open = strings(value.open);
  if (id === null || program === null || folder === null || mode === null || state === null || title === null || open === null) return null;
  if (started === null || Number.isNaN(started.getTime())) return null;
  return { id, program, folder, mode, model: str(value.model), state, session: str(value.session), title, started, ended: str(value.ended), open };
}

function questionFrom(value: unknown): Question | null {
  if (!isRecord(value)) return null;
  const key = str(value.key);
  const text = str(value.text);
  const choices = strings(value.choices);
  if (key === null || text === null || choices === null) return null;
  return { key, header: str(value.header), text, choices, many: value.many === true };
}

function askFrom(value: unknown): Ask | null {
  if (!isRecord(value)) return null;
  const kind = oneOf<RequestKind>(value.kind, KINDS);
  const tool = str(value.tool);
  const subject = str(value.subject);
  const sessionRules = strings(value.session_rules) ?? [];
  const questions = Array.isArray(value.questions) ? value.questions.map(questionFrom) : [];
  if (kind === null || tool === null || subject === null || questions.some((each) => each === null)) return null;
  return { kind, tool, subject, diff: str(value.diff), sessionRules, questions: questions as Question[] };
}

function saidFrom(value: unknown): Said | null {
  if (!isRecord(value)) return null;
  const text = str(value.text);
  const id = str(value.id);
  switch (value.kind) {
    case 'text_delta':
    case 'notice':
      return text === null ? null : { kind: value.kind, text };
    case 'message':
      return text === null || value.parent !== null ? null : { kind: 'message', text };
    case 'tool_call': {
      const name = str(value.name);
      const input = str(value.input);
      return id === null || name === null || input === null ? null : { kind: 'tool_call', id, name, input };
    }
    case 'tool_output_delta':
      return id === null || text === null ? null : { kind: 'tool_output_delta', id, text };
    case 'tool_output': {
      const output = str(value.output);
      return id === null || output === null ? null : { kind: 'tool_output', id, output, failed: value.failed === true };
    }
    case 'turn_ended': {
      const outcome = str(value.outcome);
      return outcome === null ? null : { kind: 'turn_ended', outcome, detail: str(value.detail) };
    }
    default:
      return null;
  }
}

function changeFrom(value: Fields): Change | null {
  const request = str(value.request);
  switch (value.change) {
    case 'said': {
      const event = saidFrom(value.event);
      return event === null ? null : { change: 'said', event };
    }
    case 'asked': {
      const ask = askFrom(value.detail);
      return ask === null || request === null ? null : { change: 'asked', request, ask };
    }
    case 'closed': {
      const why = str(value.why);
      return request === null || why === null ? null : { change: 'closed', request, why };
    }
    case 'prompt': {
      const text = str(value.text);
      const source = str(value.source);
      return text === null || source === null ? null : { change: 'prompt', text, source };
    }
    case 'state': {
      const state = oneOf<RunState>(value.state, STATES);
      const row = runRowFrom(value.row);
      return state === null || row === null ? null : { change: 'state', state, row };
    }
    default:
      return null;
  }
}

/** One update of the runner, or `null` for a frame that does not read or a change the page does not draw. */
export function updateFrom(value: unknown): Update | null {
  if (!isRecord(value)) return null;
  const thread = str(value.thread);
  if (thread === null || typeof value.seq !== 'number' || !Number.isSafeInteger(value.seq)) return null;
  const change = changeFrom(value);
  return change === null ? null : { thread, seq: value.seq, change };
}

/** The result of a call, or `null` when it does not read. */
export function outcomeFrom(value: unknown): Outcome | null {
  if (!isRecord(value)) return null;
  switch (value.outcome) {
    case 'ok':
      return { outcome: 'ok' };
    case 'started':
      return typeof value.thread === 'string' ? { outcome: 'started', thread: value.thread } : null;
    case 'threads': {
      const rows = Array.isArray(value.threads) ? value.threads.map(runRowFrom) : null;
      return rows === null || rows.some((row) => row === null) ? null : { outcome: 'threads', threads: rows as RunRow[] };
    }
    case 'limits': {
      const limits = value.limits;
      if (!isRecord(limits) || typeof limits.depth !== 'number' || typeof limits.children !== 'number' || typeof limits.running !== 'number') return null;
      return { outcome: 'limits', limits: { depth: limits.depth, children: limits.children, running: limits.running } };
    }
    case 'stopped':
      return { outcome: 'stopped', allEnded: value.all_ended === true };
    case 'refused': {
      const why = isRecord(value.why) ? oneOf<Refusal>(value.why.refusal, REFUSALS) : null;
      return why === null ? null : { outcome: 'refused', why };
    }
    default:
      return null;
  }
}

/** A refusal of the runner in words for the person; the same words as `anima agents`. */
export const REFUSAL_WORDS: Readonly<Record<Refusal, string>> = {
  unknown_thread: 'No thread of this computer has that ID.',
  folder_not_permitted: 'That folder is not an absolute folder of this computer.',
  program_not_found: 'That program is not on the PATH of your login.',
  bad_value: "A value is not permitted (a model name takes letters, digits, '.', '_', '-', '[' and ']').",
  request_gone: 'That request is answered, closed, or its program ended.',
  answer_does_not_fit: 'That answer does not fit the request.',
  not_running: 'The program of that thread is not running.',
  holder: 'The holder of the programs did not take the command.',
  over_limit: OVER_LIMIT,
};
