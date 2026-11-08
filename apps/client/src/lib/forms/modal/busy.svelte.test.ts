// @vitest-environment jsdom
import { flushSync, mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import type { HubResult } from '$lib/hub/hub';
import { installAnimations, played } from '../animations.fixture';
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

function confirmation(onConfirm: () => Promise<HubResult<unknown>>, onCancel: () => void, open = true) {
  const state = $state({ open });
  app = mount(ConfirmModal, {
    target: document.body,
    props: {
      get open() {
        return state.open;
      },
      title: 'Delete nas?',
      body: 'Its credentials go too.',
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

function press(key: string): void {
  (document.activeElement ?? document.body).dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true }));
  flushSync();
}

it('MOT-002: the surface uses the modal transition (scale and fade) and the scrim fades only', () => {
  const state = confirmation(() => Promise.resolve(OK), () => undefined, false);
  played.length = 0;

  state.open = true;
  flushSync();

  const keys = (selector: string): string[] =>
    played.filter((call) => call.element.matches(selector)).flatMap((call) => call.keyframes.flatMap((frame) => Object.keys(frame)));
  expect(keys('.surface')).toContain('scale');
  expect(keys('.scrim')).toContain('opacity');
  expect(keys('.scrim')).not.toContain('scale');
});

it('Modal: the dialog root is a child of body, so no other surface can clip it', () => {
  confirmation(() => Promise.resolve(OK), () => undefined);

  expect(document.querySelector('.portal')?.parentElement).toBe(document.body);
});

it('DEV-014: while busy the focus stays on the button inside the dialog, and Escape and Tab still work', () => {
  const onCancel = vi.fn();
  const onConfirm = vi.fn(() => new Promise<HubResult<unknown>>(() => undefined));
  confirmation(onConfirm, onCancel);
  const confirmButton = button('Delete nas');
  confirmButton.focus();

  confirmButton.click();
  flushSync();

  expect(confirmButton.getAttribute('aria-disabled')).toBe('true');
  expect(document.activeElement).toBe(confirmButton);
  press('Tab');
  expect(document.activeElement).toBe(button('Cancel'));
  press('Escape');
  expect(onCancel).toHaveBeenCalledOnce();
});

it('DEV-014: clicking Confirm again while busy, or after it succeeded, does not call delete again', async () => {
  const onConfirm = vi.fn(() => Promise.resolve(OK));
  confirmation(onConfirm, () => undefined);
  const confirmButton = button('Delete nas');

  confirmButton.click();
  confirmButton.click();
  await tick();
  await tick();
  confirmButton.click();
  await tick();

  expect(onConfirm).toHaveBeenCalledOnce();
});

it('SEC-007: Accept stays in the dialog while busy and is not run twice', () => {
  const onAccept = vi.fn(() => new Promise<HubResult<unknown>>(() => undefined));
  app = mount(HostKeyModal, {
    target: document.body,
    props: {
      open: true,
      hostKey: { algorithm: 'ssh-ed25519', fingerprintSha256: 'SHA256:abc', firstSeenAt: '2026-11-04T09:00:00Z' },
      changed: false,
      saved: true,
      onAccept,
      onClose: () => undefined,
    },
  });
  flushSync();
  const accept = button('Accept');
  accept.focus();

  accept.click();
  accept.click();
  flushSync();

  expect(onAccept).toHaveBeenCalledOnce();
  expect(document.activeElement).toBe(accept);
});

it('Modal: opening and closing repeatedly leaves exactly one portal while open and none after', async () => {
  const state = confirmation(() => Promise.resolve(OK), () => undefined, false);

  for (let round = 0; round < 3; round += 1) {
    state.open = true;
    flushSync();
    expect(document.querySelectorAll('.portal')).toHaveLength(1);
    state.open = false;
    flushSync();
    await tick();
    await tick();
    expect(document.querySelectorAll('.portal')).toHaveLength(0);
  }
});

it('Modal: closing, reopening mid-outro and then unmounting leaves no portal under body', () => {
  const state = confirmation(() => Promise.resolve(OK), () => undefined);

  state.open = false;
  flushSync();
  state.open = true;
  flushSync();
  if (app) void unmount(app);
  app = undefined;

  expect(document.querySelectorAll('.portal')).toHaveLength(0);
});

it('Modal: Escape is not passed on to listeners outside the dialog', () => {
  const outside = vi.fn();
  document.addEventListener('keydown', outside);
  confirmation(() => Promise.resolve(OK), () => undefined);

  press('Escape');
  document.removeEventListener('keydown', outside);

  expect(outside).not.toHaveBeenCalled();
});
