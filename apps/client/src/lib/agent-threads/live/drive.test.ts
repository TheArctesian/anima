import { describe, expect, it } from 'vitest';
import { chosenAnswer, composerKey, startOf, type NewThreadForm } from './drive';
import type { Ask } from './run';

const FORM: NewThreadForm = { program: 'codex', folder: ' /w/anima.ing ', mode: 'full_access', model: '', effort: '', prompt: 'Fix it' };

describe('the controls that drive a thread', () => {
  it('THR-017: Enter sends; Shift+Enter and a key of an input method type', () => {
    expect(composerKey({ key: 'Enter', shift: false, composing: false })).toBe('send');
    expect(composerKey({ key: 'Enter', shift: true, composing: false })).toBe('type');
    expect(composerKey({ key: 'Enter', shift: false, composing: true })).toBe('type');
    expect(composerKey({ key: 'a', shift: false, composing: false })).toBe('type');
  });

  it('THR-020: a new thread needs an absolute folder and a prompt; empty choices are the defaults', () => {
    expect(startOf(FORM)).toEqual({ ok: true, action: { call: 'start', program: 'codex', folder: '/w/anima.ing', mode: 'full_access', model: null, effort: null, prompt: 'Fix it' } });
    expect(startOf({ ...FORM, folder: 'anima.ing' }).ok).toBe(false);
    expect(startOf({ ...FORM, prompt: '  ' }).ok).toBe(false);
  });

  it('THR-030: the answers to questions go only when each question has a choice', () => {
    const ask: Ask = {
      kind: 'question',
      tool: 'AskUserQuestion',
      subject: '',
      diff: null,
      sessionRules: [],
      questions: [
        { key: 'a', header: null, text: 'Which?', choices: ['x', 'y'], many: false },
        { key: 'b', header: null, text: 'And?', choices: ['p', 'q'], many: true },
      ],
    };
    expect(chosenAnswer(ask, { a: ['x'] })).toBeNull();
    expect(chosenAnswer(ask, { a: ['x'], b: ['p', 'q'] })).toEqual({
      answer: 'choose',
      answers: [
        ['a', 'x'],
        ['b', 'p'],
        ['b', 'q'],
      ],
    });
  });
});
