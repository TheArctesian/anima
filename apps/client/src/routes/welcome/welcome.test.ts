import { describe, expect, it } from 'vitest';
import { welcomeChoices } from './welcome';

describe('the first-run choices (FRN-001)', () => {
  it('FRN-001: three choices in order; hosted sign-in is shown but not yet available', () => {
    const choices = welcomeChoices({ localHub: true, carried: true });
    expect(choices.map((choice) => choice.title)).toEqual(['Sign in', 'Join a hub', 'Create a hub on this computer']);
    expect(choices[0]).toMatchObject({ href: null, off: 'Not yet available.' });
    expect(choices[1]).toMatchObject({ href: '/welcome/join', off: null });
    expect(choices[2]).toMatchObject({ href: '/welcome/create', off: null });
  });

  it('FRN-001: a build that does not carry the hub programs keeps "Create" off and says why', () => {
    expect(welcomeChoices({ localHub: true, carried: false })[2]?.off).toBe('This build does not carry the hub programs.');
  });

  it('CLI-001: in a browser neither join nor create can be chosen', () => {
    const choices = welcomeChoices({ localHub: false, carried: false });
    expect(choices.map((choice) => choice.off !== null)).toEqual([true, true, true]);
  });
});
