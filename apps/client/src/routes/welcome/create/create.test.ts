import { describe, expect, it } from 'vitest';
import { nextStep, stepHeading, type CreateEvent, type CreateStep } from './create';

describe('the steps of "Create a hub on this computer" (FRN-002)', () => {
  it('FRN-002, FRN-004: install, account, authenticator, connect, key, invite in that order', () => {
    const events: CreateEvent[] = [{ kind: 'installed', accountExists: false }, { kind: 'account-chosen' }, { kind: 'account-made' }, { kind: 'signed-in' }, { kind: 'key-done' }];
    const visited = events.reduce<CreateStep[]>((steps, event) => [...steps, nextStep(steps.at(-1) ?? 'install', event)], ['install']);
    expect(visited).toEqual(['install', 'account', 'authenticator', 'connect', 'key', 'invite']);
  });

  it('FRN-003: when the account already exists the flow goes straight to the sign-in, then on to the key', () => {
    expect(nextStep('install', { kind: 'installed', accountExists: true })).toBe('existing');
    expect(nextStep('existing', { kind: 'signed-in' })).toBe('key');
  });

  it('FRN-004: Back from the authenticator returns to the account; there is no skip', () => {
    expect(nextStep('authenticator', { kind: 'back' })).toBe('account');
    expect(nextStep('authenticator', { kind: 'signed-in' })).toBe('authenticator');
  });

  it('an event that does not belong to the step leaves it where it is', () => {
    expect(nextStep('account', { kind: 'installed', accountExists: false })).toBe('account');
    expect(nextStep('install', { kind: 'key-done' })).toBe('install');
    expect(nextStep('invite', { kind: 'signed-in' })).toBe('invite');
  });

  it('each step has a heading and its place; the existing-account sign-in shares the connect step', () => {
    expect(stepHeading('install')).toEqual({ title: 'Install the hub', place: 'Step 1 of 6' });
    expect(stepHeading('invite').place).toBe('Step 6 of 6');
    expect(stepHeading('existing')).toEqual(stepHeading('connect'));
  });
});
