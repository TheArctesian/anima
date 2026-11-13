import { describe, expect, it } from 'vitest';
import type { HubError } from '$lib/hub/hub';
import { refusalText, validateRelease, withTarget } from '../form';

const COMMIT = '0123456789abcdef0123456789abcdef01234567';
const good = { version: '0.1.12', commit: COMMIT, targets: ['LINUX_X86_64'] } as const;

describe('release form', () => {
  it('BLD-001: a version, a full commit and one target are valid', () => {
    expect(validateRelease(good)).toEqual({});
  });

  it.each(['', '1.2', '1.2.3.4', '01.2.3', 'v1.2.3', '1.2.x', '1234567890.0.0'])('BLD-002: %j is not a version', (version) => {
    expect(validateRelease({ ...good, version }).version).toBe('A version is three numbers with dots between, such as 0.1.12.');
  });

  it.each(['', 'main', COMMIT.slice(1), `${COMMIT}0`, COMMIT.toUpperCase(), `${COMMIT.slice(1)}g`])('BLD-001: %j is not a commit', (commit) => {
    expect(validateRelease({ ...good, commit }).commit).toBe('A commit is 40 lower-case hex digits.');
  });

  it('BLD-001: at least one target is needed', () => {
    expect(validateRelease({ ...good, targets: [] }).targets).toBe('Select at least one target.');
  });

  it('BLD-001: a target is added and removed in the order the form lists them', () => {
    const order = ['LINUX_X86_64', 'DARWIN_UNIVERSAL'] as const;
    expect(withTarget(['DARWIN_UNIVERSAL'], 'LINUX_X86_64', true, order)).toEqual(['LINUX_X86_64', 'DARWIN_UNIVERSAL']);
    expect(withTarget(order, 'LINUX_X86_64', false, order)).toEqual(['DARWIN_UNIVERSAL']);
  });

  it('BLD-002: the hub refusal is its field messages, else its own sentence', () => {
    const refused: HubError = {
      code: 'VALIDATION',
      message: 'Invalid input',
      fields: [{ field: 'version', message: 'linux-x86_64 has published 0.1.12. Use a later version.' }],
    };
    expect(refusalText(refused)).toBe('linux-x86_64 has published 0.1.12. Use a later version.');
    expect(refusalText({ code: 'unreachable', message: 'The hub did not answer.' })).toBe('The hub did not answer.');
  });
});
