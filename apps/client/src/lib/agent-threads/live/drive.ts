/**
 * The rules of the controls that drive a thread (TH14, THR-017, THR-030): the keys of the composer, the words of a mode,
 * the form of a new thread, and the answers to a request. Pure.
 */

import type { Program } from '../thread';
import type { Action, Answer, Ask, Effort, Mode } from './run';

/** The words of each mode of approvals; full access is the default (decision of the operator, agent-threads.md Section 6.2). */
export const MODE_WORDS: Readonly<Record<Mode, string>> = {
  full_access: 'Full access',
  accept_edits: 'Ask before commands',
  ask: 'Ask before each change',
};

export const EFFORTS: readonly Effort[] = ['low', 'medium', 'high', 'xhigh', 'max'];

/** What a key in the composer does: Enter sends, Shift+Enter and a key of an input method type (THR-017). */
export function composerKey(press: { readonly key: string; readonly shift: boolean; readonly composing: boolean }): 'send' | 'type' {
  return press.key === 'Enter' && !press.shift && !press.composing ? 'send' : 'type';
}

/** The fields of the form of a new thread, as the person typed them. */
export interface NewThreadForm {
  readonly program: Program;
  readonly folder: string;
  readonly mode: Mode;
  readonly model: string;
  readonly effort: Effort | '';
  readonly prompt: string;
}

/** The start call of a form, or the words of what is wrong with it. The runner checks each value again. */
export function startOf(form: NewThreadForm): { readonly ok: true; readonly action: Action } | { readonly ok: false; readonly words: string } {
  const folder = form.folder.trim();
  if (!folder.startsWith('/')) return { ok: false, words: 'Give an absolute folder of this computer, for example /Users/you/project.' };
  if (form.prompt.trim() === '') return { ok: false, words: 'Type the first prompt.' };
  const model = form.model.trim();
  return {
    ok: true,
    action: { call: 'start', program: form.program, folder, mode: form.mode, model: model === '' ? null : model, effort: form.effort === '' ? null : form.effort, prompt: form.prompt },
  };
}

/** The words of the head of a request, as the program asks it. */
export function askTitle(program: string, ask: Ask): string {
  switch (ask.kind) {
    case 'command':
      return `${program} asks to run a command`;
    case 'file_change':
      return `${program} asks to change a file`;
    case 'tool':
      return `${program} asks to use ${ask.tool}`;
    case 'question':
      return `${program} asks a question`;
  }
}

/** The message to the agent when the person declines. */
export const DECLINED = 'The person declined this.';

/** The answer to the questions of a request: one choice for each question, or `null` while one has none. */
export function chosenAnswer(ask: Ask, chosen: Readonly<Record<string, readonly string[]>>): Answer | null {
  const answers: [string, string][] = [];
  for (const question of ask.questions) {
    const picks = chosen[question.key] ?? [];
    if (picks.length === 0) return null;
    for (const pick of picks) answers.push([question.key, pick]);
  }
  return { answer: 'choose', answers };
}
