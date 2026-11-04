// @vitest-environment jsdom
import { flushSync, mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import AddDeviceModal from './AddDeviceModal.svelte';
import { installAnimations } from './animations.fixture';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../motion/engine.fixture')).fakeRun,
}));

let app: ReturnType<typeof mount> | undefined;

beforeEach(() => {
  installAnimations();
});

afterEach(() => {
  if (app) void unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

function type(id: string, text: string): void {
  const field = document.getElementById(`device-${id}`) as HTMLInputElement;
  field.value = text;
  field.dispatchEvent(new Event('input', { bubbles: true }));
  flushSync();
}

it('D-6: the dialog creates the device in the only site and reports the new ID', async () => {
  const added = { id: 'd9', credentialError: null };
  const onCreate = vi.fn(() => Promise.resolve({ ok: true as const, value: added }));
  const onCreated = vi.fn();
  app = mount(AddDeviceModal, {
    target: document.body,
    props: {
      open: true,
      sites: [{ id: 's1', name: 'Home' }],
      onCreate,
      onTest: () => Promise.reject(new Error('not used')),
      onClose: () => undefined,
      onCreated,
    },
  });
  flushSync();
  expect(document.querySelector('[role="dialog"]')?.getAttribute('aria-labelledby')).not.toBeNull();

  type('name', 'nas');
  type('host', 'nas.home.arpa');
  [...document.querySelectorAll('button')].find((candidate) => candidate.textContent.trim() === 'Save')?.click();
  for (let turn = 0; turn < 4; turn += 1) await tick();

  expect(onCreate).toHaveBeenCalledOnce();
  expect(onCreate.mock.calls[0]).toEqual([expect.objectContaining({ name: 'nas', host: 'nas.home.arpa' }), 's1']);
  expect(onCreated).toHaveBeenCalledExactlyOnceWith(added);
});

it('SEC-006: the host key dialog opened from inside Add device is a child of body, not of the first dialog', async () => {
  const hostKey = { algorithm: 'ssh-ed25519', fingerprintSha256: 'SHA256:abc', firstSeenAt: '2026-11-04T09:00:00Z' };
  app = mount(AddDeviceModal, {
    target: document.body,
    props: {
      open: true,
      sites: [{ id: 's1', name: 'Home' }],
      onCreate: () => Promise.reject(new Error('not used')),
      onTest: () => Promise.resolve({ ok: true as const, value: { reachable: true, authenticated: false, hostKeyChanged: false, hostKey } }),
      onClose: () => undefined,
      onCreated: () => undefined,
    },
  });
  flushSync();
  document.querySelector<HTMLInputElement>('input[type="checkbox"]')?.click();
  flushSync();
  type('host', 'nas.home.arpa');
  type('ssh-username', 'root');

  [...document.querySelectorAll('button')].find((candidate) => candidate.textContent.trim() === 'Test connection')?.click();
  for (let turn = 0; turn < 4; turn += 1) await tick();

  const dialogs = [...document.querySelectorAll('[role="dialog"]')];
  expect(dialogs).toHaveLength(2);
  for (const dialog of dialogs) {
    expect(dialog.closest('.portal')?.parentElement).toBe(document.body);
    expect(dialog.parentElement?.closest('[role="dialog"]')).toBeNull();
  }
});
