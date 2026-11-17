import type { Answer, HostedAccount, SignInCode } from '$lib/hosted/account';
import type { HostedEvent, HostedStep } from './hosted';

/** Seconds between two asks for a hub that is starting. */
const HUB_INTERVAL = 2;

function failed(result: Answer<unknown> & { ok: false }): HostedEvent {
  return { kind: 'failed', problem: result.problem };
}

type Wait = (ms: number) => Promise<void>;

/** Polls at the interval of `code` until the person signs in or the cloud stops the sign-in. */
async function signIn(code: SignInCode, account: HostedAccount, wait: Wait): Promise<HostedEvent> {
  for (;;) {
    const polled = await account.poll(code);
    if (!polled.ok) return failed(polled);
    if (polled.value === 'signed-in') return { kind: 'signed-in' };
    await wait(code.interval * 1000);
  }
}

/** Asks for the hub until its state is not the job of `step`: the first ask is at once, a job that runs is asked again later. */
async function hubChange(step: HostedStep & { kind: 'hub' }, account: HostedAccount, wait: Wait): Promise<HostedEvent> {
  for (;;) {
    if (step.job !== null) await wait(HUB_INTERVAL * 1000);
    const hub = await account.hub();
    if (!hub.ok) return failed(hub);
    if (hub.value !== step.job) return { kind: 'hub', hub: hub.value };
  }
}

/**
 * Does the work of `step` with the cloud until something changes, and gives the event. A step that waits for
 * nothing gives `null`. `wait` takes milliseconds; a page that has gone passes a wait that never ends.
 */
export async function runStep(step: HostedStep, account: HostedAccount, wait: Wait): Promise<HostedEvent | null> {
  switch (step.kind) {
    case 'start': {
      const started = await account.start();
      if (!started.ok) return failed(started);
      const opened = await account.openPage(started.value.page);
      return { kind: 'code', code: started.value, opened: opened.ok };
    }
    case 'waiting':
      return signIn(step.code, account, wait);
    case 'hub':
      return hubChange(step, account, wait);
    case 'connecting': {
      const connected = await account.connect();
      return connected.ok ? { kind: 'connected' } : failed(connected);
    }
    case 'done':
    case 'failed':
      return null;
  }
}
