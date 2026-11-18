import { describe, expect, it } from 'vitest';
import type { Problem, SignInCode } from '$lib/hosted/account';
import { parseFailure, standInAccount, type StandInPlan } from '$lib/hosted/stand-in';
import { cloudAccount } from '$lib/desktop/cloud/shell';
import { hostedParts, type HostedParts } from '$lib/hosted/choose';
import { FIRST, nextStep, stepText, type HostedEvent, type HostedStep } from './hosted';
import { runStep } from './run';

const CODE: SignInCode = { code: 'WDJB-MJHT', page: 'https://cloud.anima.ing/activate?code=WDJB-MJHT', interval: 1 };
const WAITING: HostedStep = { kind: 'waiting', code: CODE, opened: true };
const noWait = () => Promise.resolve();

function account(plan: StandInPlan = {}): HostedParts {
  const standIn = standInAccount({ delay: 0, wait: noWait, ...plan });
  return { account: standIn, join: standIn.join };
}

/** Runs from the first step until a step waits for the person, and gives each step it passed. */
async function walk(cloud: HostedParts, from: HostedStep = FIRST): Promise<HostedStep[]> {
  const steps = [from];
  let step = from;
  for (let event = await runStep(step, cloud, noWait); event !== null; event = await runStep(step, cloud, noWait)) {
    step = nextStep(step, event);
    steps.push(step);
  }
  return steps;
}

describe('nextStep (HMD-023)', () => {
  it('HMD-023: each step of Section 4.1 in order: code, waiting, hub starting, connecting, done', () => {
    const events: HostedEvent[] = [
      { kind: 'code', code: CODE, opened: true },
      { kind: 'signed-in' },
      { kind: 'hub', hub: 'making' },
      { kind: 'hub', hub: 'ready' },
      { kind: 'connected' },
    ];
    const steps = events.reduce<HostedStep[]>((all, event) => [...all, nextStep(all.at(-1) ?? FIRST, event)], [FIRST]);
    expect(steps.map((step) => step.kind)).toEqual(['start', 'waiting', 'hub', 'hub', 'connecting', 'done']);
    expect(steps[3]).toEqual({ kind: 'hub', job: 'making' });
  });

  it('HMD-025: a hub that is ready goes straight to connecting; a sleeping one says it wakes', () => {
    const hub: HostedStep = { kind: 'hub', job: null };
    expect(nextStep(hub, { kind: 'hub', hub: 'ready' })).toEqual({ kind: 'connecting' });
    expect(stepText(nextStep(hub, { kind: 'hub', hub: 'waking' })).line).toBe('Waking your hub.');
  });

  it('HMD-043: a session that still holds goes from the start straight to the hub', () => {
    expect(nextStep(FIRST, { kind: 'signed-in' })).toEqual({ kind: 'hub', job: null });
  });

  it('HMD-043: an ended session at any step signs in again from the start', () => {
    const failed = nextStep({ kind: 'connecting' }, { kind: 'failed', problem: 'signed-out' });
    expect(stepText(failed)).toEqual({ title: 'Sign in again', line: 'Your Anima.ing session ended.', action: 'Sign in' });
    expect(nextStep(failed, { kind: 'retry' })).toEqual(FIRST);
  });

  it('an event that does not belong to the step leaves it; done stays done', () => {
    expect(nextStep(FIRST, { kind: 'connected' })).toBe(FIRST);
    expect(nextStep({ kind: 'done' }, { kind: 'failed', problem: 'offline' })).toEqual({ kind: 'done' });
  });

  it('the browser opened again, or not, changes only the line of the waiting step', () => {
    const closed = nextStep(WAITING, { kind: 'opened', opened: false });
    expect(closed).toEqual({ ...WAITING, opened: false });
    expect(stepText(closed).line).toBe('The browser did not open. Open the page, then check the code.');
  });

  const retries: [HostedStep, Problem, HostedStep][] = [
    [FIRST, 'offline', FIRST],
    [FIRST, 'refused', FIRST],
    [WAITING, 'cancelled', FIRST],
    [WAITING, 'expired', FIRST],
    [WAITING, 'refused', FIRST],
    [WAITING, 'offline', WAITING],
    [{ kind: 'hub', job: 'making' }, 'offline', { kind: 'hub', job: null }],
    [{ kind: 'connecting' }, 'refused', { kind: 'connecting' }],
  ];
  it.each(retries)('HMD-023: %o stopped by %s shows why, and Retry goes to %o', (from, problem, back) => {
    const failed = nextStep(from, { kind: 'failed', problem });
    expect(failed).toEqual({ kind: 'failed', problem, from });
    expect(stepText(failed).action).not.toBeNull();
    expect(nextStep(failed, { kind: 'retry' })).toEqual(back);
  });

  it('HMD-023: each failure has its own words; a refusal by the hub names the hub', () => {
    const text = (problem: Problem, from: HostedStep = FIRST) => stepText({ kind: 'failed', problem, from });
    expect(text('cancelled')).toEqual({ title: 'Sign-in cancelled', line: 'The sign-in was cancelled in the browser.', action: 'Try again' });
    expect(text('expired')).toEqual({ title: 'The code expired', line: 'Sign in before the code runs out.', action: 'Get a new code' });
    expect(text('offline')).toEqual({ title: 'No network', line: 'Anima.ing did not answer. Check the network.', action: 'Retry' });
    expect(text('refused').line).toBe('Anima.ing refused the sign-in.');
    expect(text('refused', { kind: 'connecting' }).line).toBe('Your hub refused this computer.');
  });
});

describe('runStep with the stand-in', () => {
  it('HMD-023, HMD-025: walks from the code to done, making the hub on the way', async () => {
    const steps = await walk(account({ polls: 2, starts: 2 }));
    expect(steps.map((step) => step.kind)).toEqual(['start', 'waiting', 'hub', 'hub', 'connecting', 'done']);
    expect(steps[1]).toEqual({ kind: 'waiting', code: expect.objectContaining({ code: 'WDJB-MJHT' }) as SignInCode, opened: true });
  });

  it('HMD-025: a ready hub skips the starting step', async () => {
    expect((await walk(account({ hub: 'ready' }))).map((step) => step.kind)).toEqual(['start', 'waiting', 'hub', 'connecting', 'done']);
  });

  it('polls at the interval of the code until the sign-in', async () => {
    const waits: number[] = [];
    const event = await runStep(WAITING, account({ polls: 3 }), (ms) => {
      waits.push(ms);
      return Promise.resolve();
    });
    expect(event).toEqual({ kind: 'signed-in' });
    expect(waits).toEqual([1000, 1000, 1000]);
  });

  it('a browser that does not open still shows the code', async () => {
    expect(await runStep(FIRST, account({ fail: { at: 'open', problem: 'refused' } }), noWait)).toMatchObject({ kind: 'code', opened: false });
  });

  const stops: [StandInPlan['fail'] & object, HostedStep['kind']][] = [
    [{ at: 'start', problem: 'offline' }, 'start'],
    [{ at: 'poll', problem: 'cancelled' }, 'waiting'],
    [{ at: 'poll', problem: 'expired' }, 'waiting'],
    [{ at: 'hub', problem: 'refused' }, 'hub'],
    [{ at: 'access', problem: 'refused' }, 'connecting'],
    [{ at: 'join', problem: 'offline' }, 'connecting'],
  ];
  it.each(stops)('HMD-023: a failure at %o stops in %s, and one retry finishes', async (fail, at) => {
    const cloud = account({ fail });
    const steps = await walk(cloud);
    const stopped = steps.at(-1);
    expect(stopped).toMatchObject({ kind: 'failed', problem: fail.problem, from: { kind: at } });
    const after = await walk(cloud, nextStep(stopped ?? FIRST, { kind: 'retry' }));
    expect(after.at(-1)).toEqual({ kind: 'done' });
  });
});

describe('the cloud of the first run', () => {
  it('reads a failure as step:problem and ignores anything else', () => {
    expect(parseFailure('poll:expired')).toEqual({ at: 'poll', problem: 'expired' });
    expect(parseFailure('poll:gone')).toBeNull();
    expect(parseFailure(null)).toBeNull();
  });

  it('HMD-043: outside the desktop shell the real cloud has no sign-in', async () => {
    expect(await walk({ account: cloudAccount(), join: () => Promise.reject(new Error('not reached')) })).toEqual([FIRST, { kind: 'failed', problem: 'unconfigured', from: FIRST }]);
  });

  it('only VITE_ANIMA_HOSTED=stand-in picks the stand-in, and ?fail tells it where to fail', async () => {
    const query = new URLSearchParams('fail=start:offline');
    expect(await hostedParts('stand-in', query).account.start()).toEqual({ ok: false, problem: 'offline' });
    expect(await hostedParts(undefined, query).account.start()).toEqual({ ok: false, problem: 'unconfigured' });
  });
});
