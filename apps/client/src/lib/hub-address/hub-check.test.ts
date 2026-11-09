import { expect, it } from 'vitest';
import { TOKEN_UNREADABLE, UNREACHABLE } from '$lib/hub/hub';
import { connectionSentence } from './hub-check';

it('CLI-002: an unreadable token says nothing was sent, never that the hub answered', () => {
  const sentence = connectionSentence({ ok: false, error: TOKEN_UNREADABLE });
  expect(sentence).toBe('Anima cannot read the token.');
  expect(sentence.length).toBeLessThanOrEqual(30);
});

it('CLI-002: a refusal from the hub still shows that the address reaches a hub', () => {
  expect(connectionSentence({ ok: false, error: { code: 'UNAUTHENTICATED', message: 'sign in' } })).toBe('The hub answered.');
  expect(connectionSentence({ ok: true, value: null })).toBe('The hub answered.');
});

it('CLI-002: silence and unreadable answers are failures', () => {
  expect(connectionSentence({ ok: false, error: UNREACHABLE })).toBe('The hub did not answer.');
  expect(connectionSentence({ ok: false, error: { code: 'malformed', message: 'unreadable' } })).toBe('The hub sent an odd answer.');
});
