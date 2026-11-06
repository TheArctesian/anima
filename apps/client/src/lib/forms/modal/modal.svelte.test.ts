// @vitest-environment jsdom
import { flushSync, mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import type { HubResult } from '$lib/hub/error';
import { installAnimations } from '../animations.fixture';
import ConfirmModal from './ConfirmModal.svelte';
import HostKeyModal from './HostKeyModal.svelte';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../motion/engine.fixture')).fakeRun,
}));

const OK: HubResult<unknown> = { ok: true, value: undefined };
let app: ReturnType<typeof mount> | undefined;

beforeEach(() => {
  installAnimations();
});

afterEach(() => {
  if (app) void unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

function opener(): HTMLButtonElement {
  const button = document.createElement('button');
  button.textContent = 'Delete device';
  document.body.append(button);
  button.focus();
  return button;
}

function confirmation(onConfirm: () => Promise<HubResult<unknown>>, onCancel: () => void) {
  const state = $state({ open: true });
  app = mount(ConfirmModal, {
    target: document.body,
    props: {
      get open() {
        return state.open;
      },
      title: 'Delete nas?',
      body: 'Its credentials, checks and history go too.',
      confirmLabel: 'Delete nas',
      onConfirm,
      onCancel,
    },
  });
  flushSync();
  return state;
}

function button(label: string): HTMLButtonElement {
  const found = [...document.querySelectorAll('button')].find((candidate) => candidate.textContent.trim() === label);
  if (!found) throw new Error(`no button ${label}`);
  return found;
}

function press(key: string, init: KeyboardEventInit = {}): void {
  const target = document.activeElement ?? document.body;
  target.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...init }));
  flushSync();
}

it('DEV-014: the dialog is labelled, modal, and Cancel has the focus', () => {
  opener();
  confirmation(() => Promise.resolve(OK), () => undefined);

  const dialog = document.querySelector('[role="dialog"]');
  expect(dialog?.getAttribute('aria-modal')).toBe('true');
  const title = document.getElementById(dialog?.getAttribute('aria-labelledby') ?? '');
  expect(title?.textContent).toBe('Delete nas?');
  expect(document.activeElement).toBe(button('Cancel'));
});

it('DEV-014: Cancel and Escape do not call delete', () => {
  const onConfirm = vi.fn(() => Promise.resolve(OK));
  const onCancel = vi.fn();
  opener();
  confirmation(onConfirm, onCancel);

  button('Cancel').click();
  press('Escape');

  expect(onCancel).toHaveBeenCalledTimes(2);
  expect(onConfirm).not.toHaveBeenCalled();
});

it('DEV-014: Confirm calls delete once, shows a busy state, and a second click does nothing', async () => {
  let finish: (result: HubResult<unknown>) => void = () => undefined;
  const onConfirm = vi.fn(() => new Promise<HubResult<unknown>>((done) => { finish = done; }));
  opener();
  confirmation(onConfirm, () => undefined);

  button('Delete nas').click();
  flushSync();
  expect(document.querySelector('[aria-busy="true"]')).not.toBeNull();
  document.querySelector<HTMLButtonElement>('[aria-busy="true"]')?.click();
  finish(OK);
  await tick();

  expect(onConfirm).toHaveBeenCalledOnce();
});

it('DEV-014: a hub error is shown and Confirm can be tried again', async () => {
  const failure: HubResult<unknown> = { ok: false, error: { code: 'UNAVAILABLE', message: 'The hub is busy.' } };
  const onConfirm = vi.fn(() => Promise.resolve(failure));
  opener();
  confirmation(onConfirm, () => undefined);

  button('Delete nas').click();
  await tick();
  await tick();

  expect(document.querySelector('[role="alert"]')?.textContent).toBe('The hub is busy.');
  button('Delete nas').click();
  expect(onConfirm).toHaveBeenCalledTimes(2);
});

it('Modal: Tab stays inside the dialog, forwards and backwards', () => {
  const outside = opener();
  confirmation(() => Promise.resolve(OK), () => undefined);

  button('Delete nas').focus();
  press('Tab');
  expect(document.activeElement).toBe(button('Cancel'));
  press('Tab', { shiftKey: true });
  expect(document.activeElement).toBe(button('Delete nas'));
  expect(document.activeElement).not.toBe(outside);
});

it('Modal: focus returns to the opener when the dialog closes, without waiting for the outro', () => {
  const trigger = opener();
  const state = confirmation(() => Promise.resolve(OK), () => undefined);
  expect(document.activeElement).toBe(button('Cancel'));

  state.open = false;
  flushSync();

  expect(document.activeElement).toBe(trigger);
});

it('Modal: after Escape the focus returns to the clicked button, also when the click did not focus it (WebKit)', () => {
  // A tab panel that holds the focus while its button is clicked; jsdom, like WebKit, does not focus a clicked button.
  const panel = document.createElement('div');
  panel.tabIndex = 0;
  const trigger = document.createElement('button');
  trigger.textContent = 'Delete device';
  panel.append(trigger);
  document.body.append(panel);
  panel.focus();
  const state = $state({ open: false });
  app = mount(ConfirmModal, {
    target: document.body,
    props: {
      get open() {
        return state.open;
      },
      title: 'Delete nas?',
      body: 'Its credentials, checks and history go too.',
      confirmLabel: 'Delete nas',
      onConfirm: () => Promise.resolve(OK),
      onCancel: () => {
        state.open = false;
      },
    },
  });
  flushSync();
  trigger.addEventListener('click', () => {
    state.open = true;
  });

  trigger.click();
  flushSync();
  expect(document.activeElement).toBe(button('Cancel'));
  press('Escape');

  expect(document.activeElement).toBe(trigger);
});

it('Modal: Tab reaches every button of the dialog in order, not only the two ends', () => {
  opener();
  confirmation(() => Promise.resolve(OK), () => undefined);
  const order = [document.activeElement?.textContent.trim()];

  press('Tab');
  order.push(document.activeElement?.textContent.trim());
  press('Tab');
  order.push(document.activeElement?.textContent.trim());

  expect(order).toEqual(['Cancel', 'Delete nas', 'Cancel']);
});

it('Modal: mounted dialogs, open or closed, share one click listener, and it is gone after they unmount', () => {
  const add = vi.spyOn(document, 'addEventListener');
  const remove = vi.spyOn(document, 'removeEventListener');
  const clicks = (spy: typeof add) => spy.mock.calls.filter(([type, , options]) => type === 'click' && typeof options === 'object' && options.capture === true).length;
  const props = { open: false, title: 'Delete nas?', body: 'Gone.', confirmLabel: 'Delete nas', onConfirm: () => Promise.resolve(OK), onCancel: () => undefined };
  const first = mount(ConfirmModal, { target: document.body, props });
  const second = mount(ConfirmModal, { target: document.body, props: { ...props, open: true } });
  flushSync();
  expect(clicks(add)).toBe(1);

  void unmount(first);
  void unmount(second);
  flushSync();

  expect(clicks(remove)).toBe(1);
  add.mockRestore();
  remove.mockRestore();
});

it('Modal, MOT-042: closing uses no timer', () => {
  const timers = vi.spyOn(globalThis, 'setTimeout');
  opener();
  const state = confirmation(() => Promise.resolve(OK), () => undefined);
  timers.mockClear();

  state.open = false;
  flushSync();

  expect(timers).not.toHaveBeenCalled();
  timers.mockRestore();
});

function hostKey(changed: boolean, saved: boolean, onAccept: (fingerprint: string) => Promise<HubResult<unknown>>): void {
  app = mount(HostKeyModal, {
    target: document.body,
    props: {
      open: true,
      hostKey: { algorithm: 'ssh-ed25519', fingerprintSha256: 'SHA256:abc123', firstSeenAt: '2026-11-04T09:00:00Z' },
      changed,
      saved,
      onAccept,
      onClose: () => undefined,
    },
  });
  flushSync();
}

it('SEC-006: the modal shows the algorithm, the SHA-256 fingerprint in mono and when it was first seen', () => {
  hostKey(false, true, () => Promise.resolve(OK));

  const text = document.querySelector('[role="dialog"]')?.textContent ?? '';
  expect(text).toContain('ssh-ed25519');
  expect(text).toContain('2026-11-04T09:00:00Z');
  expect(document.querySelector('.fingerprint')?.textContent).toBe('SHA256:abc123');
});

it('SEC-007: a changed key is a warning with the fixed text', () => {
  hostKey(true, true, () => Promise.resolve(OK));

  expect(document.querySelector('[role="alert"]')?.textContent).toBe(
    'The host key changed. Do not trust this device until you check it.',
  );
});

it('D-5: Accept sends the fingerprint for a saved device', () => {
  const onAccept = vi.fn(() => Promise.resolve(OK));
  hostKey(false, true, onAccept);

  button('Accept').click();

  expect(onAccept).toHaveBeenCalledExactlyOnceWith('SHA256:abc123');
});

it('D-5: an unsaved device has no Accept button and says the key can be accepted after saving', () => {
  hostKey(false, false, () => Promise.resolve(OK));

  expect(() => button('Accept')).toThrow();
  expect(document.querySelector('[role="dialog"]')?.textContent).toContain('Save the device, then test again to accept this key.');
});
