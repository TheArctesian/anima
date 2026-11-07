// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { runs } from '$lib/motion/engine.fixture';
import { SystemSettingsError } from '$lib/desktop/systemSettings';
import type { CardDevice } from '$lib/devices/api';
import DeviceCard from '../card/DeviceCard.svelte';
import { device as cardDevice } from '../screen/screen.fixture.svelte';
import { endTransitionsAtOnce } from '../transitions.fixture';
import { answer, fakeCalls, settle, UPDATED } from './calls.fixture';
import ScreenSharingButton from './ScreenSharingButton.svelte';
import { NO_ANSWER_TEXT, type CardScreenSharing } from './setup';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));
const preference = vi.hoisted(() => ({ reduced: false }));
vi.mock('$lib/motion/preference', () => ({ motionPreference: () => preference }));

endTransitionsAtOnce();

let app: ReturnType<typeof mount> | undefined;

beforeEach(() => {
  runs.length = 0;
  preference.reduced = false;
});

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

const mac: CardDevice = cardDevice('a', { name: 'Studio', host: '100.64.0.5', os: 'MACOS' });

function showCard(over: Partial<CardDevice>, screenSharing: CardScreenSharing | undefined) {
  app = mount(DeviceCard, {
    target: document.body,
    props: { device: { ...mac, ...over }, beats: undefined, onConnect: vi.fn(), screenSharing },
  });
  flushSync();
}

function button(name: RegExp | string): HTMLButtonElement | undefined {
  return [...document.querySelectorAll('button')].find((node) =>
    typeof name === 'string'
      ? node.textContent.replace(/\p{Co}/gu, '').trim() === name
      : name.test(node.getAttribute('aria-label') ?? node.textContent),
  );
}

function press(target: HTMLButtonElement | undefined): void {
  target?.click();
}

async function flow(): Promise<void> {
  await settle();
  flushSync();
}

function mountButton(compact = false, calls = fakeCalls()) {
  const onReady = vi.fn();
  app = mount(ScreenSharingButton, {
    target: document.body,
    props: { device: { id: 'd1', name: 'Studio', host: '100.64.0.5', os: 'MACOS', vnc: { enabled: false } }, calls: calls.calls, onReady, compact },
  });
  flushSync();
  return { ...calls, onReady };
}

it('R-079: a card shows "Screen sharing" with the accessible name "Set up screen sharing for <name>"', () => {
  showCard({}, { calls: fakeCalls().calls, onReady: vi.fn() });
  const found = button('Screen sharing');
  expect(found?.getAttribute('aria-label')).toBe('Set up screen sharing for Studio');
  expect(found?.tabIndex).toBe(0);
});

it.each([
  ['WINDOWS', false],
  ['OTHER', false],
  ['MACOS', true],
])('R-079: a card of %s with VNC on=%s has no screen sharing button', (os, enabled) => {
  showCard({ os: os as CardDevice['os'], vnc: { enabled } }, { calls: fakeCalls().calls, onReady: vi.fn() });
  expect(button(/screen sharing/i)).toBeUndefined();
});

it('R-079: a card shows no button while no route gives the calls', () => {
  showCard({}, undefined);
  expect(button(/screen sharing/i)).toBeUndefined();
});

it('R-079: the full text shows where there is room', () => {
  mountButton();
  expect(button('Set up screen sharing')).toBeDefined();
});

it('R-079: a device that answers hands the updated device over and the status line clears', async () => {
  const t = mountButton();
  t.calls.test.mockResolvedValue(answer(true));
  press(button(/screen sharing/i));
  await flow();
  expect(t.onReady).toHaveBeenCalledWith(UPDATED, expect.any(HTMLButtonElement));
  expect(document.querySelector('[role="status"]')).toBeNull();
});

it('R-079: this Mac not answering shows "Turn on Screen Sharing" in a status region with Cancel; Cancel stops the repeats', async () => {
  const t = mountButton();
  press(button(/screen sharing/i));
  await flow();
  expect(document.querySelector('[role="status"]')?.textContent).toContain('Turn on Screen Sharing');
  expect(t.calls.openSharing).toHaveBeenCalledTimes(1);
  press(button('Cancel'));
  await flow();
  expect(t.clock.waiting()).toBe(0);
  expect(document.querySelector('[role="status"]')).toBeNull();
});

it('R-079: unmounting while waiting stops the repeats', async () => {
  const t = mountButton();
  press(button(/screen sharing/i));
  await flow();
  expect(t.clock.waiting()).toBe(1);
  await unmount(app ?? {});
  app = undefined;
  expect(t.clock.waiting()).toBe(0);
});

it('R-079: a failure to open System Settings shows its fixed message and offers a retry', async () => {
  const t = mountButton(false, fakeCalls({ openSharing: vi.fn(() => Promise.reject(new SystemSettingsError('failed'))) }));
  press(button(/screen sharing/i));
  await flow();
  expect(document.querySelector('[role="alert"]')?.textContent).toBe('System Settings did not open.');
  press(button('Open System Settings again'));
  await flow();
  expect(t.calls.openSharing).toHaveBeenCalledTimes(2);
});

it('R-079: another device that does not answer shows the failure as an alert, and Close dismisses it', async () => {
  mountButton(false, fakeCalls({ thisMachine: () => Promise.resolve(null) }));
  press(button(/screen sharing/i));
  await flow();
  expect(document.querySelector('[role="alert"]')?.textContent).toBe(NO_ANSWER_TEXT);
  press(button('Close'));
  await flow();
  expect(document.querySelector('[role="alert"]')).toBeNull();
});

it('R-079: a failed update shows its message and hands nothing over', async () => {
  const t = mountButton();
  t.calls.test.mockResolvedValue(answer(true));
  t.calls.update.mockResolvedValue({ ok: false, error: { code: 'INTERNAL', message: 'The hub could not save.' } });
  press(button(/screen sharing/i));
  await flow();
  expect(document.querySelector('[role="alert"]')?.textContent).toBe('The hub could not save.');
  expect(t.onReady).not.toHaveBeenCalled();
});

it('MOT-040: while waiting, the indicator fades without reduced motion and stays still with it', async () => {
  mountButton();
  press(button(/screen sharing/i));
  await flow();
  expect(runs.some((run) => run.keyframes.at(-1) === 0.4)).toBe(true);
  await unmount(app ?? {});
  document.body.replaceChildren();
  runs.length = 0;
  preference.reduced = true;
  app = undefined;
  mountButton();
  press(button(/screen sharing/i));
  await flow();
  expect(document.querySelector('.dot')).not.toBeNull();
  expect(runs).toEqual([]);
});
