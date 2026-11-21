/**
 * A thread that the runner drives, as the page shows it: the updates of the runner folded into a timeline, the requests
 * that wait, and the row (TH14, THR-014, THR-030). Pure: the time of each update is a parameter.
 */

import type { Entry, Program, ToolCall, ToolKind } from '../thread';
import type { Ask, RunRow, Said, Update } from './run';

/** The most characters of the output of one tool call that the page keeps. */
export const OUTPUT_CHARS = 20_000;
/** The most entries of one thread that the page keeps; the oldest go first. */
export const ENTRY_LIMIT = 2_000;

/** A part of the timeline of a driven thread: an entry of a session, or a notice of the program. */
export type LiveEntry =
  | Exclude<Entry, { kind: 'tool' }>
  | { readonly kind: 'tool'; readonly at: Date; readonly tool: ToolCall; readonly call: string }
  | { readonly kind: 'notice'; readonly at: Date; readonly text: string };

/** A request that waits for the person, with the runner's ID for the answer. */
export interface OpenAsk {
  readonly request: string;
  readonly ask: Ask;
}

export interface LiveThread {
  readonly row: RunRow;
  readonly entries: readonly LiveEntry[];
  /** The reply as it comes, before the program sends it whole. */
  readonly streaming: string;
  readonly asks: readonly OpenAsk[];
  /** The highest sequence number taken: an update at or below it is a repeat. */
  readonly seq: number;
  /** When the last update came. */
  readonly last: Date;
  /** How many entries went past the limit. */
  readonly dropped: number;
}

/** The kind of a tool by its name, for the words and the icon of the block. */
export function toolKind(name: string): ToolKind {
  const lower = name.toLowerCase();
  if (['bash', 'command', 'commandexecution', 'shell'].includes(lower)) return 'command';
  if (['edit', 'write', 'multiedit', 'notebookedit', 'filechange'].includes(lower)) return 'edit';
  if (['read', 'ls'].includes(lower)) return 'read';
  if (['grep', 'glob', 'search'].includes(lower)) return 'search';
  if (['webfetch', 'websearch'].includes(lower)) return 'web';
  if (['task', 'agent'].includes(lower)) return 'agent';
  if (lower === 'todowrite') return 'plan';
  if (lower === 'askuserquestion') return 'question';
  return 'other';
}

/** The most characters of one text of the program that the page keeps: a reply, a prompt, a tool input, a request. */
export const TEXT_CHARS = 200_000;

function held(text: string): string {
  return text.length > TEXT_CHARS ? `${text.slice(0, TEXT_CHARS)}…` : text;
}

function capped(text: string): string {
  return text.length > OUTPUT_CHARS ? `${text.slice(0, OUTPUT_CHARS)}…` : text;
}

function heldAsk(ask: Ask): Ask {
  return { ...ask, subject: capped(ask.subject), diff: ask.diff === null ? null : held(ask.diff) };
}

/** A new thread from its first row. */
export function begun(row: RunRow, at: Date): LiveThread {
  return { row, entries: [], streaming: '', asks: [], seq: 0, last: at, dropped: 0 };
}

function withEntry(thread: LiveThread, entry: LiveEntry): LiveThread {
  const entries = [...thread.entries, entry];
  const over = Math.max(0, entries.length - ENTRY_LIMIT);
  return { ...thread, entries: over > 0 ? entries.slice(over) : entries, dropped: thread.dropped + over };
}

/** The entries with the tool call `call` changed by `change`; the newest call of that ID is the one. */
function withTool(entries: readonly LiveEntry[], call: string, change: (tool: ToolCall) => ToolCall): readonly LiveEntry[] {
  const at = entries.findLastIndex((entry) => entry.kind === 'tool' && entry.call === call);
  const entry = entries[at];
  if (entry?.kind !== 'tool') return entries;
  return entries.with(at, { ...entry, tool: change(entry.tool) });
}

function said(thread: LiveThread, event: Said, at: Date): LiveThread {
  switch (event.kind) {
    case 'text_delta':
      return { ...thread, streaming: capped(thread.streaming + event.text) };
    case 'message':
      return { ...withEntry(thread, { kind: 'reply', at, text: held(event.text) }), streaming: '' };
    case 'tool_call': {
      const firstLine = event.input.split('\n', 1)[0] ?? '';
      const tool: ToolCall = { name: event.name, kind: toolKind(event.name), summary: capped(firstLine), input: capped(event.input), patch: null, output: null, failed: false };
      return withEntry(thread, { kind: 'tool', at, tool, call: event.id });
    }
    case 'tool_output_delta':
      return { ...thread, entries: withTool(thread.entries, event.id, (tool) => ({ ...tool, output: capped((tool.output ?? '') + event.text) })) };
    case 'tool_output':
      return { ...thread, entries: withTool(thread.entries, event.id, (tool) => ({ ...tool, output: capped(event.output), failed: event.failed })) };
    case 'turn_ended':
      return { ...withEntry(thread, { kind: 'turn_end', at, stopped: event.outcome === 'interrupted' }), streaming: '' };
    case 'notice':
      return withEntry(thread, { kind: 'notice', at, text: capped(event.text) });
  }
}

/**
 * `thread` with `update` taken at `at`. An update at or below the last sequence number is a repeat (the runner sends the
 * updates it keeps again to each new watch) and changes nothing. A request leaves the list only when the runner closes
 * it: by an answer, a stop or the end of its process (THR-030).
 */
export function applied(thread: LiveThread, update: Update, at: Date): LiveThread {
  if (update.seq <= thread.seq) return thread;
  const next = { ...thread, seq: update.seq, last: at };
  const change = update.change;
  switch (change.change) {
    case 'said':
      return said(next, change.event, at);
    case 'asked':
      return { ...next, asks: [...next.asks.filter((each) => each.request !== change.request), { request: change.request, ask: heldAsk(change.ask) }] };
    case 'closed':
      return { ...next, asks: next.asks.filter((each) => each.request !== change.request) };
    case 'prompt':
      return withEntry(next, { kind: 'prompt', at, text: held(change.text), images: 0 });
    case 'state':
      return { ...next, row: change.row, asks: next.asks.filter((each) => change.row.open.includes(each.request)) };
  }
}

/** The tool calls that follow each other as one block, as the timeline of a session file draws them. */
export type LivePart = { readonly kind: 'entry'; readonly entry: Exclude<LiveEntry, { kind: 'tool' }> } | { readonly kind: 'tools'; readonly tools: readonly ToolCall[] };

export function liveParts(entries: readonly LiveEntry[]): readonly LivePart[] {
  const out: LivePart[] = [];
  let run: ToolCall[] = [];
  for (const entry of entries) {
    if (entry.kind === 'tool') {
      run.push(entry.tool);
      continue;
    }
    if (run.length > 0) out.push({ kind: 'tools', tools: run });
    run = [];
    out.push({ kind: 'entry', entry });
  }
  if (run.length > 0) out.push({ kind: 'tools', tools: run });
  return out;
}

/** The time the runner gives a program for its first answer before it ends it (TH97); the same as the runner. */
export const FIRST_ANSWER_MINUTES = 10;

/**
 * What a thread that is starting waits for, with the time it has waited (TH97): on a busy computer Claude Code reads its
 * sign-in from the Keychain and Codex loads its plugins first, which can take minutes.
 */
export function waitingWords(program: Program, started: Date, now: Date): string {
  const seconds = Math.max(0, Math.floor((now.getTime() - started.getTime()) / 1000));
  const waited = seconds < 60 ? `${String(seconds)} s` : `${String(Math.floor(seconds / 60))} min ${String(seconds % 60)} s`;
  const first = program === 'claude_code' ? 'Claude Code reads its sign-in first' : 'Codex loads its plugins first';
  return `Waiting for the first answer of the program: ${waited}. ${first}; on a busy computer this can take minutes. The runner ends it after ${String(FIRST_ANSWER_MINUTES)} minutes with no answer.`;
}
