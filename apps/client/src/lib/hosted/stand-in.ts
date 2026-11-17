import { sealAccess } from './access';
import { answer, refusal, type Answer, type HostedAccount, type HubState, type Problem } from './account';

/** A step of the stand-in that can be told to fail once. `access` is the cloud's answer with the hub access; `join` is the link join of this computer. */
export type StandInStep = 'start' | 'open' | 'poll' | 'hub' | 'access' | 'join';

export interface StandInFailure {
  readonly at: StandInStep;
  readonly problem: Problem;
}

export interface StandInPlan {
  /** Fails this step once; the retry goes through. */
  readonly fail?: StandInFailure | null;
  /** What the cloud says about the hub on the first ask. */
  readonly hub?: HubState;
  /** Polls that answer `waiting` before the sign-in completes. */
  readonly polls?: number;
  /** Asks that answer `making` or `waking` before the hub is ready. */
  readonly starts?: number;
  /** The delay of each call, in milliseconds; tests pass 0. */
  readonly delay?: number;
  readonly wait?: (ms: number) => Promise<void>;
}

const STEPS: readonly StandInStep[] = ['start', 'open', 'poll', 'hub', 'access', 'join'];
const PROBLEMS: readonly Problem[] = ['cancelled', 'expired', 'offline', 'refused'];

/** Reads a failure written as `step:problem`, for example `poll:expired`; anything else is no failure. */
export function parseFailure(text: string | null): StandInFailure | null {
  const [at, problem] = (text ?? '').split(':');
  const step = STEPS.find((known) => known === at);
  const found = PROBLEMS.find((known) => known === problem);
  return step === undefined || found === undefined ? null : { at: step, problem: found };
}

/**
 * A cloud and a link join that are not there: it walks through the steps of the sign-in with delays like the real
 * ones, and fails where the plan says. It holds no token and talks to no network. It is for tests and for `VITE_ANIMA_HOSTED=stand-in`.
 */
export function standInAccount(plan: StandInPlan = {}): HostedAccount & { readonly join: () => Promise<Answer<void>> } {
  const { hub = 'making', polls = 3, starts = 3, delay = 600, wait = (ms) => new Promise((done) => setTimeout(done, ms)) } = plan;
  let failure = plan.fail ?? null;
  let pollsLeft = polls;
  let startsLeft = hub === 'ready' ? 0 : starts;

  /** Waits like a network call, then fails once if the plan says so. */
  async function call(step: StandInStep): Promise<Problem | null> {
    await wait(delay);
    if (failure?.at !== step) return null;
    const { problem } = failure;
    failure = null;
    return problem;
  }

  return {
    start: async () => {
      const problem = await call('start');
      pollsLeft = polls;
      return problem === null ? answer({ code: 'WDJB-MJHT', page: 'https://cloud.anima.ing/activate?code=WDJB-MJHT', interval: 1 }) : refusal(problem);
    },
    openPage: async () => {
      const problem = await call('open');
      return problem === null ? answer(undefined) : refusal(problem);
    },
    poll: async () => {
      const problem = await call('poll');
      if (problem !== null) return refusal(problem);
      if (pollsLeft === 0) return answer('signed-in');
      pollsLeft -= 1;
      return answer('waiting');
    },
    hub: async () => {
      const problem = await call('hub');
      if (problem !== null) return refusal(problem);
      if (startsLeft === 0) return answer('ready');
      startsLeft -= 1;
      return answer(hub);
    },
    // A sealed access with no secret in it: the stand-in join never opens it.
    access: async () => {
      const problem = await call('access');
      return problem === null ? answer(sealAccess({ link: 'stand-in', token: 'stand-in' })) : refusal(problem);
    },
    /** The link join of the stand-in: it joins nothing and keeps nothing. */
    join: async () => {
      const problem = await call('join');
      return problem === null ? answer(undefined) : refusal(problem);
    },
  };
}
