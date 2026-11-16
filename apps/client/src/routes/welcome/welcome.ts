/** One first-run choice: what it is, one line about it, and why it cannot be chosen when it cannot (FRN-001). */
export interface Choice {
  readonly id: 'sign-in' | 'join' | 'create';
  readonly title: string;
  readonly line: string;
  readonly href: '/welcome/join' | '/welcome/create' | null;
  /** Why the choice is off, or `null` when it is on. */
  readonly off: string | null;
}

/** What decides the choices: the desktop app's local hub support, and whether this build carries the hub programs. */
export interface ChoicePlace {
  readonly localHub: boolean;
  readonly carried: boolean;
}

/** The three choices of the first run, in order (FRN-001). The hosted sign-in has no service yet. */
export function welcomeChoices(place: ChoicePlace): readonly Choice[] {
  return [
    { id: 'sign-in', title: 'Sign in', line: 'Use the hub that Anima.ing hosts for you.', href: null, off: 'Not yet available.' },
    {
      id: 'join',
      title: 'Join a hub',
      line: 'Connect this computer to a hub with an invite from it.',
      href: '/welcome/join',
      off: place.localHub ? null : 'Only the desktop app can join a hub.',
    },
    {
      id: 'create',
      title: 'Create a hub on this computer',
      line: 'This computer runs the hub, its database and its link. Other devices join it.',
      href: '/welcome/create',
      off: !place.localHub ? 'Only the desktop app can run a hub.' : place.carried ? null : 'This build does not carry the hub programs.',
    },
  ];
}
