import { describe, expect, it } from 'vitest';
import { welcomeChoices } from './welcome';

describe('the first-run choices (FRN-001)', () => {
  it('FRN-001, HMD-022: three choices in order; hosted sign-in is the one primary choice', () => {
    const choices = welcomeChoices({ deviceLink: true, localHub: true, carried: true });
    expect(choices.map((choice) => choice.title)).toEqual(['Sign in', 'Join a hub', 'Create a hub on this computer']);
    expect(choices.map((choice) => choice.primary)).toEqual([true, false, false]);
    expect(choices[0]).toMatchObject({ href: '/welcome/hosted', off: null });
    expect(choices[1]).toMatchObject({ href: '/welcome/join', off: null });
    expect(choices[2]).toMatchObject({ href: '/welcome/create', off: null });
  });

  it('FRN-001: a build that does not carry the hub programs keeps "Create" off and says why', () => {
    expect(welcomeChoices({ deviceLink: true, localHub: true, carried: false })[2]?.off).toBe('This build does not carry the hub programs.');
  });

  it('R-129: a phone with the link in the app can sign in, but cannot join or create a hub here', () => {
    const choices = welcomeChoices({ deviceLink: true, localHub: false, carried: false });
    expect(choices.map((choice) => choice.off)).toEqual([null, 'Desktop app only.', 'Desktop app only.']);
  });

  it('CLI-001: in a browser no choice can be chosen: each needs the desktop app', () => {
    const choices = welcomeChoices({ deviceLink: false, localHub: false, carried: false });
    expect(choices.map((choice) => choice.off !== null)).toEqual([true, true, true]);
  });
});
