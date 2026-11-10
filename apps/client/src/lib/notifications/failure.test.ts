import { describe, expect, it } from 'vitest';
import { UNREACHABLE } from '$lib/hub/hub';
import { failureText } from './failure';
import { FAKE_SECRET, failure } from './notifications.fixture';

function errorOf(result: ReturnType<typeof failure>) {
  if (result.ok) throw new Error('not a failure');
  return result.error;
}

describe('failureText', () => {
  it('MOT-007: what a refused save says fits the line kept in the editor footer at 390 px (35 characters)', () => {
    const codes = ['VALIDATION', 'CONFLICT', 'NOT_FOUND', 'FORBIDDEN', 'UNAVAILABLE', 'INTERNAL', 'UNAUTHENTICATED'] as const;
    const sentences = codes.map((code) => failureText(errorOf(failure(code)), 'save'));
    expect(sentences.filter((sentence) => sentence.length > 35)).toEqual([]);
  });

  it('LOG-007, F-8: the hub text is never shown, whatever the code', () => {
    for (const code of ['VALIDATION', 'CONFLICT', 'NOT_FOUND', 'FORBIDDEN', 'UNAVAILABLE', 'INTERNAL', 'UNAUTHENTICATED'] as const) {
      for (const action of ['save', 'delete', 'test', 'settings'] as const) {
        expect(failureText(errorOf(failure(code, `refused ${FAKE_SECRET}`, [{ field: 'url', message: FAKE_SECRET }])), action)).not.toContain(FAKE_SECRET);
      }
    }
  });

  it('LOG-007: a VALIDATION error names the refused fields by the form\'s labels only', () => {
    const fields = [
      { field: 'input.settings.webhook.url', message: FAKE_SECRET },
      { field: 'input.template.body', message: 'x' },
      { field: 'constructor', message: 'x' },
      { field: FAKE_SECRET, message: 'x' },
    ];
    expect(failureText(errorOf(failure('VALIDATION', 'x', fields)), 'save')).toBe('The hub refused: URL, Template body.');
    expect(failureText(errorOf(failure('VALIDATION', 'x', [{ field: '__proto__', message: 'x' }])), 'save')).toBe('The hub refused what was entered.');
  });

  it('NET-004: an unreachable hub keeps the client\'s own sentence', () => {
    expect(failureText(UNREACHABLE, 'test')).toBe(UNREACHABLE.message);
  });

  it('NOT-004: a name conflict on save asks for another name; elsewhere the action\'s own sentence', () => {
    expect(failureText(errorOf(failure('CONFLICT')), 'save')).toBe('Name in use. Choose another.');
    expect(failureText(errorOf(failure('INTERNAL')), 'test')).toBe('Anima could not send the test. Try again.');
  });
});
