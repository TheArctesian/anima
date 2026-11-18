import { describe, expect, it } from 'vitest';
import { hubChoices, signInChoice } from './welcome';

describe('the first-run choices (FRN-001)', () => {
  it('HMD-022: the first screen has one choice, the hosted sign-in', () => {
    expect(signInChoice({ deviceLink: true, localHub: true, carried: true }, true)).toMatchObject({ title: 'Sign in', href: '/welcome/hosted', off: null });
  });

  it('CLI-001: in a browser the hosted sign-in needs the app', () => {
    expect(signInChoice({ deviceLink: false, localHub: false, carried: false }, false).off).toBe('Needs the Anima.ing app.');
  });

  it('M3-1: a phone app without the link says that sign-in is not on phones yet', () => {
    expect(signInChoice({ deviceLink: false, localHub: false, carried: false }, true).off).toBe('Not on phones yet.');
  });

  it('FRN-001, HMD-022: behind "Join a hub" are the invite, the hub address and the hub on this computer, in order', () => {
    const choices = hubChoices({ deviceLink: true, localHub: true, carried: true });
    expect(choices.map((choice) => [choice.title, choice.href, choice.off])).toEqual([
      ['Join with an invite', '/welcome/join', null],
      ['Use a hub address', '/sign-in', null],
      ['Create a hub on this computer', '/welcome/create', null],
    ]);
  });

  it('FRN-001: a build that does not carry the hub programs keeps "Create" off and says why', () => {
    expect(hubChoices({ deviceLink: true, localHub: true, carried: false })[2]?.off).toBe('This build does not carry the hub programs.');
  });

  it('R-129: a phone or a browser can use a hub address, but cannot join with an invite or create a hub here', () => {
    const choices = hubChoices({ deviceLink: false, localHub: false, carried: false });
    expect(choices.map((choice) => choice.off)).toEqual(['Desktop app only.', null, 'Desktop app only.']);
  });
});
