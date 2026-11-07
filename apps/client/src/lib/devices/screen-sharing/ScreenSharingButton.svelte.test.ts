// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { SystemSettingsError } from '$lib/desktop/systemSettings';
import type { CardDevice } from '$lib/devices/api';
import DeviceCard from '../card/DeviceCard.svelte';
import DeviceTable from '../card/DeviceTable.svelte';
import { device as cardDevice } from '../screen/screen.fixture.svelte';
import { endTransitionsAtOnce } from '../transitions.fixture';
import { answer, fakeCalls, settle, UPDATED } from './calls.fixture';
import ScreenSharingButton from './ScreenSharingButton.svelte';
import { NO_ANSWER_TEXT, type CardScreenSharing } from './setup';

vi.mock('$lib/motion/preference', () => ({ motionPreference: () => ({ reduced: false }) }));

endTransitionsAtOnce();

let app: ReturnType<typeof mount> | undefined;

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

function mountButton(compact = false) {
  const calls = fakeCalls();
  const onReady = vi.fn();
  app = mount(ScreenSharingButton, {
    target: document.body,
    props: { device: { id: 'd1', name: 'Studio', host: '100.64.0.5', os: 'MACOS', vnc: { enabled: false } }, calls: calls.calls, onReady, compact },
  });
  flushSync();
  return { ...calls, onReady };
}

it('R-079: a card shows "VNC", the width of the VNC button, with the name "Set up screen sharing for <name>" and a title', () => {
  showCard({}, { calls: fakeCalls().calls, onReady: vi.fn() });
  const found = button('VNC');
  expect(found?.title).toBe('Set up screen sharing');
  expect(found?.classList.contains('dashed')).toBe(true);
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

it('R-079: the header form holds the full text and "VNC", and the container query shows one of them', () => {
  mountButton();
  expect(document.querySelector('.full')?.textContent).toBe('Set up screen sharing');
  expect(document.querySelector('.short')?.textContent).toBe('VNC');
});

it('R-079: the compact form holds only "VNC"', () => {
  mountButton(true);
  expect(document.querySelector('.full')).toBeNull();
});

const status = () => document.querySelector('[role="status"]')?.textContent;

it('R-079: a device that answers hands the updated device over and the status line clears', async () => {
  const t = mountButton();
  t.calls.test.mockResolvedValue(answer(true));
  press(button(/screen sharing/i));
  await flow();
  expect(t.onReady).toHaveBeenCalledWith(UPDATED, expect.any(HTMLButtonElement));
  expect(status()).toBe('');
});

it('R-079: this Mac not answering shows "Turn on Screen Sharing" in a status region with Cancel; Cancel stops the repeats', async () => {
  const t = mountButton();
  press(button(/screen sharing/i));
  await flow();
  expect(status()).toContain('Turn on Screen Sharing');
  expect(t.calls.openSharing).toHaveBeenCalledTimes(1);
  press(button('Cancel'));
  await flow();
  expect(t.clock.waiting()).toBe(0);
  expect(status()).toBe('');
  expect(document.activeElement).toBe(button(/screen sharing/i));
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
  const t = mountButton();
  t.calls.openSharing.mockRejectedValueOnce(new SystemSettingsError('failed'));
  press(button(/screen sharing/i));
  await flow();
  expect(document.querySelector('[role="alert"]')?.textContent).toBe('System Settings did not open.');
  press(button('Open System Settings again'));
  await flow();
  expect(t.calls.openSharing).toHaveBeenCalledTimes(2);
});

it('R-079: another device that does not answer shows the failure as an alert, and Close dismisses it', async () => {
  const t = mountButton();
  t.calls.thisMachine.mockResolvedValue(null);
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

it('NAV-005: the button stays focusable while the flow runs, so focus is not lost', async () => {
  mountButton();
  const target = button(/screen sharing/i);
  target?.focus();
  press(target);
  await flow();
  expect(target?.disabled).toBe(false);
  expect(target?.getAttribute('aria-disabled')).toBe('true');
  expect(document.activeElement).toBe(target);
});

it('R-079: Escape inside the button cancels the waiting; Escape elsewhere, or already used, does not', async () => {
  const t = mountButton();
  press(button(/screen sharing/i));
  await flow();
  const outside = document.createElement('input');
  document.body.append(outside);
  outside.focus();
  outside.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
  expect(t.clock.waiting()).toBe(1);
  button('Cancel')?.focus();
  const used = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true });
  document.addEventListener('keydown', (event) => { event.preventDefault(); }, { once: true, capture: true });
  document.activeElement?.dispatchEvent(used);
  expect(t.clock.waiting()).toBe(1);
  document.activeElement?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }));
  await flow();
  expect(t.clock.waiting()).toBe(0);
  expect(document.activeElement).toBe(button(/screen sharing/i));
});

it('R-079: waiting has no repeating motion', async () => {
  mountButton();
  press(button(/screen sharing/i));
  await flow();
  expect(document.querySelector('.dot')).toBeNull();
});

function showTable(over: Partial<CardDevice>, withCalls: boolean) {
  app = mount(DeviceTable, {
    target: document.body,
    props: {
      devices: [{ ...mac, ...over }],
      beats: new Map(),
      onConnect: vi.fn(),
      screenSharing: withCalls ? { calls: fakeCalls().calls, onReady: vi.fn() } : undefined,
    },
  });
  flushSync();
}

it('R-079: a table row of a macOS device with VNC off shows "VNC" set-up; Windows, VNC on, or no calls hide it', async () => {
  showTable({}, true);
  expect(button('VNC')?.getAttribute('aria-label')).toBe('Set up screen sharing for Studio');
  await unmount(app ?? {});
  document.body.replaceChildren();
  for (const [over, withCalls] of [[{ os: 'WINDOWS' as const }, true], [{ vnc: { enabled: true } }, true], [{}, false]] as const) {
    showTable(over, withCalls);
    expect(button(/set up screen sharing/i)).toBeUndefined();
    await unmount(app ?? {});
    document.body.replaceChildren();
  }
  app = undefined;
});

it('R-079: Cancel while the update is in flight lets it finish but opens no session', async () => {
  const t = mountButton();
  t.calls.test.mockResolvedValue(answer(true));
  let finish: (value: Awaited<ReturnType<typeof t.calls.update>>) => void = () => undefined;
  t.calls.update.mockReturnValue(new Promise((resolve) => { finish = resolve; }));
  button(/screen sharing/i)?.focus();
  press(button(/screen sharing/i));
  await flow();
  // Starting has no Cancel button; Escape is the way out, and it stops the hand-over only.
  document.activeElement?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }));
  finish({ ok: true, value: UPDATED });
  await flow();
  expect(t.calls.update).toHaveBeenCalledTimes(1);
  expect(t.onReady).not.toHaveBeenCalled();
});
