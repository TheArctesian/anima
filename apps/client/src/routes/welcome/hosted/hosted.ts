import type { HubState, Problem, SignInCode } from '$lib/hosted/account';

/**
 * The steps of "Sign in" (hosted-mode.md Section 4.1, HMD-023): get a code, wait for the sign-in in the browser,
 * the hub starts (make or wake), connect to it, then the device list. The person never types an invite, an address or
 * a hub password: the app joins by itself (Section 4.3). A step that stops shows why and offers a retry.
 */
export type HostedStep =
  | { readonly kind: 'start' }
  | { readonly kind: 'waiting'; readonly code: SignInCode; readonly opened: boolean }
  /** `job` is null while the app asks the cloud for the hub. */
  | { readonly kind: 'hub'; readonly job: Exclude<HubState, 'ready'> | null }
  | { readonly kind: 'connecting' }
  | { readonly kind: 'done' }
  | { readonly kind: 'failed'; readonly problem: Problem; readonly from: HostedStep };

export type HostedEvent =
  | { readonly kind: 'code'; readonly code: SignInCode; readonly opened: boolean }
  | { readonly kind: 'opened'; readonly opened: boolean }
  | { readonly kind: 'signed-in' }
  | { readonly kind: 'hub'; readonly hub: HubState }
  | { readonly kind: 'connected' }
  | { readonly kind: 'failed'; readonly problem: Problem }
  | { readonly kind: 'retry' };

export const FIRST: HostedStep = { kind: 'start' };

/** Where Retry goes: a code that is cancelled, expired or refused is spent, so the sign-in starts again. */
function retried(problem: Problem, from: HostedStep): HostedStep {
  if (problem === 'signed-out') return FIRST;
  if (from.kind === 'waiting' && problem !== 'offline') return FIRST;
  if (from.kind === 'hub') return { kind: 'hub', job: null };
  return from;
}

/** The step after a sign-in step: the browser opened again, or the person signed in. */
function afterWaiting(step: HostedStep & { kind: 'waiting' }, event: HostedEvent): HostedStep {
  if (event.kind === 'opened') return { ...step, opened: event.opened };
  return event.kind === 'signed-in' ? { kind: 'hub', job: null } : step;
}

/** The step after the cloud said where the hub is: ready to connect, or a job that runs. */
function afterHub(step: HostedStep, event: HostedEvent): HostedStep {
  if (event.kind !== 'hub') return step;
  return event.hub === 'ready' ? { kind: 'connecting' } : { kind: 'hub', job: event.hub };
}

/** The step after `event` in `step`; an event that does not belong to the step leaves it where it is. */
export function nextStep(step: HostedStep, event: HostedEvent): HostedStep {
  if (event.kind === 'failed') return step.kind === 'done' || step.kind === 'failed' ? step : { kind: 'failed', problem: event.problem, from: step };
  switch (step.kind) {
    case 'start':
      if (event.kind === 'signed-in') return { kind: 'hub', job: null };
      return event.kind === 'code' ? { kind: 'waiting', code: event.code, opened: event.opened } : step;
    case 'waiting':
      return afterWaiting(step, event);
    case 'hub':
      return afterHub(step, event);
    case 'connecting':
      return event.kind === 'connected' ? { kind: 'done' } : step;
    case 'failed':
      return event.kind === 'retry' ? retried(step.problem, step.from) : step;
    case 'done':
      return step;
  }
}

/** The words of a step: its heading, one line, and the label of its button when it has one. */
export interface StepText {
  readonly title: string;
  readonly line: string;
  readonly action: string | null;
}

const JOB_LINE: Readonly<Record<Exclude<HubState, 'ready'> | 'asking', string>> = {
  asking: 'Finding your hub.',
  making: 'Making your hub. This can take a minute.',
  waking: 'Waking your hub.',
};

const PROBLEM_TEXT: Readonly<Record<Problem, StepText>> = {
  cancelled: { title: 'Sign-in cancelled', line: 'The sign-in was cancelled in the browser.', action: 'Try again' },
  expired: { title: 'The code expired', line: 'Sign in before the code runs out.', action: 'Get a new code' },
  offline: { title: 'No network', line: 'Anima.ing did not answer. Check the network.', action: 'Retry' },
  refused: { title: 'Sign-in refused', line: 'Anima.ing refused the sign-in.', action: 'Retry' },
  'signed-out': { title: 'Sign in again', line: 'Your Anima.ing session ended.', action: 'Sign in' },
  unconfigured: { title: 'Sign-in not available', line: 'This build of Anima.ing has no account sign-in.', action: 'Retry' },
  payment: { title: 'Payment failed', line: 'Pay on cloud.anima.ing, then retry.', action: 'Retry' },
};

/** The words of a failed step: a refusal by the hub names the hub, and a sign-in closed inside the app names no browser. */
function failedText(step: Extract<HostedStep, { kind: 'failed' }>, here: string, inApp: boolean): StepText {
  if (step.problem === 'refused' && step.from.kind === 'connecting') return { ...PROBLEM_TEXT.refused, line: `Your hub refused ${here}.` };
  if (inApp && step.problem === 'cancelled') return { ...PROBLEM_TEXT.cancelled, line: 'The sign-in was closed.' };
  return PROBLEM_TEXT[step.problem];
}

/**
 * The words of `step`. `here` names what signs in: a phone says "this device" (MOB-016). With `inApp` the sign-in
 * page shows inside the app, so no step names a code or a browser (mobile plan M3-5).
 */
export function stepText(step: HostedStep, here: 'this computer' | 'this device' = 'this computer', inApp = false): StepText {
  switch (step.kind) {
    case 'start':
      return { title: 'Starting', line: inApp ? 'Opening the sign-in.' : 'Getting a code.', action: null };
    case 'waiting':
      return {
        title: 'Continue in your browser',
        line: step.opened ? 'Check that the browser shows this code.' : 'The browser did not open. Open the page, then check the code.',
        action: 'Open the page',
      };
    case 'hub':
      return { title: 'Your hub is starting', line: JOB_LINE[step.job ?? 'asking'], action: null };
    case 'connecting':
      return { title: 'Connecting', line: `Connecting ${here} to your hub.`, action: null };
    case 'done':
      return { title: 'Your hub is ready', line: 'Opening your devices.', action: null };
    case 'failed':
      return failedText(step, here, inApp);
  }
}
