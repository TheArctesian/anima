/** One way to reach a hub: what it is, one line about it, and why it cannot be chosen when it cannot (FRN-001). */
export interface Choice {
  readonly id: 'sign-in' | 'join' | 'address' | 'create';
  readonly title: string;
  readonly line: string;
  readonly href: '/welcome/hosted' | '/welcome/join' | '/welcome/address' | '/sign-in' | '/welcome/create';
  /** Why the choice is off, or `null` when it is on. */
  readonly off: string | null;
}

/**
 * What decides the choices: whether the shell joins this device over the link (the hosted sign-in), the desktop app's
 * local hub support, and whether this build carries the hub programs.
 */
export interface ChoicePlace {
  readonly deviceLink: boolean;
  /** The shell signs in to the Anima.ing account (its cloud commands). */
  readonly cloudAccount: boolean;
  readonly localHub: boolean;
  readonly carried: boolean;
}

/**
 * The one choice of the first run's first screen: sign in with the Anima.ing account (HMD-022). It needs a shell that
 * joins over the link and has the cloud commands: the desktop app's or a phone's (R-129, mobile plan M3-4). A browser
 * has neither. The other ways are behind "Join a hub" (operator, 2026-11-17: "sign in with SSO and then one link at the bottom to
 * join a hub").
 */
export function signInChoice(place: ChoicePlace): Choice {
  return {
    id: 'sign-in',
    title: 'Sign in',
    line: 'With your Anima.ing account.',
    href: '/welcome/hosted',
    off: place.deviceLink && place.cloudAccount ? null : 'Needs the Anima.ing app.',
  };
}

/** The ways to reach a hub of one's own, in order, on the page behind "Join a hub" (FRN-001, HMD-022). */
export function hubChoices(place: ChoicePlace): readonly Choice[] {
  return [
    {
      id: 'join',
      title: 'Join with an invite',
      line: 'Link this device to a hub with an invite.',
      href: '/welcome/join',
      // The desktop shell runs anima-link; a phone runs the link inside the app (MOB-031, M3-1).
      off: place.deviceLink ? null : 'Needs the Anima.ing app.',
    },
    {
      id: 'address',
      title: place.deviceLink ? 'Use a hub address' : 'Sign in to this hub',
      line: place.deviceLink ? 'Enter the address of a hub your team runs.' : 'For a different hub, open its address in your browser.',
      href: place.deviceLink ? '/welcome/address' : '/sign-in',
      off: null,
    },
    {
      id: 'create',
      title: 'Create a hub on this computer',
      line: 'This computer runs the hub. Other devices join it.',
      href: '/welcome/create',
      off: !place.localHub ? 'Desktop app only.' : place.carried ? null : 'This build does not carry the hub programs.',
    },
  ];
}
