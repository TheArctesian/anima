/**
 * The steps of "Create a hub on this computer" (FRN-002): install, then the account and its authenticator, then connect
 * and sign in, then the copy of the hub key, then the first invite. `existing` is the connect step for a hub whose
 * account already exists: it asks for that account's name and password, signs in, then goes on to the key.
 */
export type CreateStep = 'install' | 'account' | 'authenticator' | 'connect' | 'existing' | 'key' | 'invite';

export type CreateEvent =
  | { readonly kind: 'installed'; readonly accountExists: boolean }
  | { readonly kind: 'account-chosen' }
  | { readonly kind: 'back' }
  | { readonly kind: 'account-made' }
  | { readonly kind: 'signed-in' }
  | { readonly kind: 'key-done' };

/** The step after `event` in `step`; an event that does not belong to the step leaves it where it is. */
export function nextStep(step: CreateStep, event: CreateEvent): CreateStep {
  if (event.kind === 'installed') {
    if (step !== 'install') return step;
    return event.accountExists ? 'existing' : 'account';
  }
  switch (`${step}:${event.kind}`) {
    case 'account:account-chosen':
      return 'authenticator';
    case 'authenticator:back':
      return 'account';
    case 'authenticator:account-made':
      return 'connect';
    case 'connect:signed-in':
    case 'existing:signed-in':
      return 'key';
    case 'key:key-done':
      return 'invite';
    default:
      return step;
  }
}

const TITLES: Readonly<Record<CreateStep, string>> = {
  install: 'Install the hub',
  account: 'Make your account',
  authenticator: 'Set up your authenticator app',
  connect: 'Sign in to your hub',
  existing: 'Sign in to your hub',
  key: 'Back up the hub key',
  invite: 'Invite your next device',
};

const ORDER: readonly CreateStep[] = ['install', 'account', 'authenticator', 'connect', 'key', 'invite'];

/** The step's heading and its place, for example "Step 2 of 6". */
export function stepHeading(step: CreateStep): { readonly title: string; readonly place: string } {
  const index = step === 'existing' ? ORDER.indexOf('connect') : ORDER.indexOf(step);
  return { title: TITLES[step], place: `Step ${String(index + 1)} of ${String(ORDER.length)}` };
}
