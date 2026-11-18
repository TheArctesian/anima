/** One way to reach a hub: what it is, one line about it, and why it cannot be chosen when it cannot (FRN-001). */
export interface Choice {
  readonly id: 'sign-in' | 'join' | 'address' | 'create';
  readonly title: string;
  readonly line: string;
  readonly href: '/welcome/hosted' | '/welcome/join' | '/sign-in' | '/welcome/create';
  /** Why the choice is off, or `null` when it is on. */
  readonly off: string | null;
}

/**
 * What decides the choices: whether the shell joins this device over the link (the hosted sign-in), the desktop app's
 * local hub support, and whether this build carries the hub programs.
 */
export interface ChoicePlace {
  readonly deviceLink: boolean;
  readonly localHub: boolean;
  readonly carried: boolean;
}

/**
 * The one choice of the first run's first screen: sign in with the Anima.ing account (HMD-022). It needs a shell that
 * joins over the link, the desktop app's or a phone's (R-129); `inApp` is true in an app of Anima.ing (its key store).
 * The other ways are behind "Join a hub" (operator, 2026-11-17: "sign in with SSO and then one link at the bottom to
 * join a hub").
 */
export function signInChoice(place: ChoicePlace, inApp: boolean): Choice {
  return {
    id: 'sign-in',
    title: 'Sign in',
    line: 'With your Anima.ing account.',
    href: '/welcome/hosted',
    // The phone app is the app; it signs in once it runs the link (mobile plan M3-1).
    off: place.deviceLink ? null : inApp ? 'Not on phones yet.' : 'Needs the Anima.ing app.',
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
      off: place.localHub ? null : 'Desktop app only.',
    },
    {
      id: 'address',
      title: 'Use a hub address',
      line: 'Sign in to a hub on a server of your own.',
      href: '/sign-in',
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
