/** Frames of the socket of the runner, as the desktop shell passes them, and a made shell, for the tests. */

import type { RunnerShell } from '$lib/desktop/agent-threads/runner';
import { updateFrom, type Action, type Outcome, type Update } from './run';

export const THREAD = '0190a000-0000-7000-8000-000000000001';

export function rowFrame(state: string, more: Record<string, unknown> = {}): Record<string, unknown> {
  return { id: THREAD, program: 'claude_code', folder: '/w/anima.ing', mode: 'full_access', model: null, state, session: null, title: 'Fix the test', started: '2026-11-20T10:00:00Z', ended: null, open: [], ...more };
}

export function frame(seq: number, change: Record<string, unknown>): Update {
  const update = updateFrom({ thread: THREAD, seq, ...change });
  if (update === null) throw new Error('the frame does not read');
  return update;
}

export const stateFrame = (seq: number, state: string, more: Record<string, unknown> = {}): Update => frame(seq, { change: 'state', state, row: rowFrame(state, more) });

export const askFrame = (seq: number, request: string, subject = 'bun test'): Update =>
  frame(seq, { change: 'asked', request, detail: { id: 'r-1', kind: 'command', tool: 'Bash', subject, diff: null, session_rules: ['Bash(bun test:*)'], questions: [] } });

export const saidFrame = (seq: number, event: Record<string, unknown>): Update => frame(seq, { change: 'said', event });

/** A made shell: each call is kept, and the answers are given by `answer`. */
export function madeShell(answer: (action: Action) => Outcome = () => ({ outcome: 'ok' })): RunnerShell & {
  readonly calls: Action[];
  readonly notices: [string, string][];
  emit: (update: Update) => void;
  link: (linked: boolean) => void;
} {
  const calls: Action[] = [];
  const notices: [string, string][] = [];
  let onUpdate: (update: Update) => void = () => undefined;
  let onLink: (linked: boolean) => void = () => undefined;
  return {
    calls,
    notices,
    emit: (update) => {
      onUpdate(update);
    },
    link: (linked) => {
      onLink(linked);
    },
    call: (action) => {
      calls.push(action);
      return Promise.resolve(answer(action));
    },
    watch: (update, link) => {
      onUpdate = update;
      onLink = link;
      return Promise.resolve(() => undefined);
    },
    notify: (title, words) => {
      notices.push([title, words]);
      return Promise.resolve();
    },
  };
}
