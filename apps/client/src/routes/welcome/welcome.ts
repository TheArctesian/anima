/** One first-run choice: what it is, one line about it, and why it cannot be chosen when it cannot (FRN-001). */
export interface Choice {
  readonly id: 'sign-in' | 'join' | 'create';
  readonly title: string;
  readonly line: string;
  readonly href: '/welcome/hosted' | '/welcome/join' | '/welcome/create' | null;
  /** The one main choice of the first run (HMD-022). */
  readonly primary: boolean;
  /** Why the choice is off, or `null` when it is on. */
  readonly off: string | null;
}

/** What decides the choices: the desktop app's local hub support, and whether this build carries the hub programs. */
export interface ChoicePlace {
  readonly localHub: boolean;
  readonly carried: boolean;
}

/**
 * The three choices of the first run, in order (FRN-001). "Sign in" is the primary choice (HMD-022); it needs the
 * link of the desktop app, as "Join" does.
 */
export function welcomeChoices(place: ChoicePlace): readonly Choice[] {
  return [
    {
      id: 'sign-in',
      title: 'Sign in',
      line: 'Use a hub hosted by Anima.ing.',
      href: '/welcome/hosted',
      primary: true,
      off: place.localHub ? null : 'Desktop app only.',
    },
    {
      id: 'join',
      title: 'Join a hub',
      line: 'Link this computer to a hub with an invite.',
      href: '/welcome/join',
      primary: false,
      off: place.localHub ? null : 'Desktop app only.',
    },
    {
      id: 'create',
      title: 'Create a hub on this computer',
      line: 'This computer runs the hub. Other devices join it.',
      href: '/welcome/create',
      primary: false,
      off: !place.localHub ? 'Desktop app only.' : place.carried ? null : 'This build does not carry the hub programs.',
    },
  ];
}
