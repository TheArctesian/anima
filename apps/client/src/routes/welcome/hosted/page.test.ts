// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installAnimations } from '$lib/forms/animations.fixture';
import { standInAccount, type StandInPlan } from '$lib/hosted/stand-in';
import HostedSignIn from './HostedSignIn.svelte';

const mocks = vi.hoisted(() => ({ loadClientAt: vi.fn(), done: vi.fn() }));

vi.mock('$app/paths', () => ({ resolve: (path: string) => path, asset: (path: string) => path }));
vi.mock('$lib/access/navigation', () => ({ loadClientAt: mocks.loadClientAt }));

let app: ReturnType<typeof mount> | undefined;
/** Each wait of the page is held here until the test lets it go, so the test sees each step. */
let held: (() => void)[] = [];

async function settle(): Promise<void> {
  for (let round = 0; round < 4; round += 1) {
    await new Promise((resolve) => setTimeout(resolve, 0));
    flushSync();
  }
}

async function open(plan: StandInPlan = {}): Promise<{ target: HTMLElement; account: ReturnType<typeof standInAccount> }> {
  const account = standInAccount({ delay: 0, wait: () => Promise.resolve(), polls: 1, starts: 1, ...plan });
  const target = document.createElement('div');
  document.body.append(target);
  const wait = () => new Promise<void>((resolve) => held.push(resolve));
  app = mount(HostedSignIn, { target, props: { parts: { account, join: account.join }, wait, ondone: mocks.done } });
  await settle();
  return { target, account };
}

/** Lets each held wait go, then settles. */
async function release(): Promise<void> {
  const waiting = held;
  held = [];
  for (const go of waiting) go();
  await settle();
}

function view(target: HTMLElement) {
  const step = [...target.querySelectorAll('.step')].at(-1);
  return {
    title: step?.querySelector('h2')?.textContent,
    line: step?.querySelector('.line')?.textContent,
    code: step?.querySelector('.code')?.textContent ?? null,
    buttons: [...(step?.querySelectorAll('button') ?? [])].map((button) => button.textContent.trim()),
    back: step?.querySelector('a')?.getAttribute('href') ?? null,
  };
}

function press(target: HTMLElement, label: string): void {
  [...target.querySelectorAll('.step button')].find((button) => button.textContent.trim() === label)?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
}

beforeEach(() => {
  installAnimations();
  mocks.loadClientAt.mockReset();
  mocks.done.mockReset();
  held = [];
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});

it('HMD-023: shows the code to compare while it waits for the sign-in in the browser', async () => {
  const { target } = await open();
  expect(view(target)).toEqual({
    title: 'Continue in your browser',
    line: 'Check that the browser shows this code.',
    code: 'WDJB-MJHT',
    buttons: ['Open the page'],
    back: '/welcome',
  });
  expect(document.activeElement?.textContent).toBe('Continue in your browser');
});

it('HMD-023, HMD-025, Section 4.3: after the sign-in the hub starts, the app joins by itself, then the device list opens', async () => {
  const { target } = await open();
  await release();
  expect(view(target)).toMatchObject({ title: 'Your hub is starting', line: 'Making your hub. This can take a minute.', buttons: [], back: null });
  expect(mocks.done).not.toHaveBeenCalled();
  await release();
  expect(view(target)).toMatchObject({ title: 'Your hub is ready', line: 'Opening your devices.', buttons: [] });
  expect(mocks.done).toHaveBeenCalledOnce();
  expect(target.querySelector('input, textarea')).toBeNull();
});

it('without ondone the end of the flow loads the device list', async () => {
  const account = standInAccount({ delay: 0, wait: () => Promise.resolve(), polls: 0, hub: 'ready' });
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(HostedSignIn, { target, props: { parts: { account, join: account.join }, wait: () => Promise.resolve() } });
  await settle();
  expect(mocks.loadClientAt).toHaveBeenCalledWith('/devices');
});

it('HMD-025: a sleeping hub says that it wakes', async () => {
  const { target } = await open({ hub: 'waking' });
  await release();
  expect(view(target).line).toBe('Waking your hub.');
});

it('the start step says that it gets a code', async () => {
  const account = standInAccount({ delay: 0, wait: () => new Promise(() => undefined) });
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(HostedSignIn, { target, props: { parts: { account, join: account.join } } });
  flushSync();
  expect(view(target)).toMatchObject({ title: 'Starting', line: 'Getting a code.', buttons: [], back: '/welcome' });
});

it('a browser that did not open says so, and "Open the page" tries again', async () => {
  const { target, account } = await open({ fail: { at: 'open', problem: 'refused' } });
  expect(view(target).line).toBe('The browser did not open. Open the page, then check the code.');
  const opened = vi.spyOn(account, 'openPage');
  press(target, 'Open the page');
  await settle();
  expect(opened).toHaveBeenCalledWith('https://cloud.anima.ing/activate?code=WDJB-MJHT');
  expect(view(target).line).toBe('Check that the browser shows this code.');
});

const failures: [StandInPlan['fail'] & object, string, string, string][] = [
  [{ at: 'poll', problem: 'cancelled' }, 'Sign-in cancelled', 'The sign-in was cancelled in the browser.', 'Try again'],
  [{ at: 'poll', problem: 'expired' }, 'The code expired', 'Sign in before the code runs out.', 'Get a new code'],
  [{ at: 'start', problem: 'offline' }, 'No network', 'Anima.ing did not answer. Check the network.', 'Retry'],
  [{ at: 'start', problem: 'refused' }, 'Sign-in refused', 'Anima.ing refused the sign-in.', 'Retry'],
  [{ at: 'join', problem: 'refused' }, 'Sign-in refused', 'Your hub refused this computer.', 'Retry'],
  [{ at: 'join', problem: 'offline' }, 'No network', 'Anima.ing did not answer. Check the network.', 'Retry'],
];

it.each(failures)('HMD-023: a failure %o says why with an alert, and its button finishes the sign-in', async (fail, title, line, action) => {
  const { target } = await open({ fail, hub: 'ready' });
  await release();
  expect(view(target)).toMatchObject({ title, line, buttons: [action], back: '/welcome' });
  expect(target.querySelector('.step [role="alert"]')?.textContent).toBe(line);
  press(target, action);
  await settle();
  await release();
  await release();
  expect(view(target).title).toBe('Your hub is ready');
  expect(mocks.done).toHaveBeenCalledOnce();
});
