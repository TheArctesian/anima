// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { endTransitionsAtOnce } from '$lib/devices/transitions.fixture';
import { answer, fakeCalls, settle, UPDATED } from '$lib/devices/screen-sharing/calls.fixture';
import { device } from '../page.fixture.svelte';
import Header from './Header.svelte';
import type { HeaderDevice } from './header';

endTransitionsAtOnce();

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

function show(shown: HeaderDevice, withCalls = true) {
  const fake = fakeCalls();
  const onConnect = vi.fn<(protocol: string, button: HTMLElement) => void>();
  const onEnabled = vi.fn();
  app = mount(Header, {
    target: document.body,
    props: {
      device: shown,
      onConnect,
      onSettings: vi.fn(),
      screenSharing: withCalls ? { calls: fake.calls, onEnabled } : undefined,
    },
  });
  flushSync();
  return { ...fake, onConnect, onEnabled };
}

const setUp = () =>
  [...document.querySelectorAll('button')].find((node) => node.getAttribute('aria-label')?.startsWith('Set up screen sharing'));

it.each(['MACOS', 'LINUX'] as const)('R-079: a %s device with VNC off has the dashed "Set up screen sharing" button in the VNC slot', (os) => {
  show(device({ os }));
  const found = setUp();
  expect(found?.textContent.replace(/\p{Co}/gu, '').trim()).toBe('Set up screen sharing');
  expect(found?.classList.contains('dashed')).toBe(true);
  expect(found?.getAttribute('aria-label')).toBe('Set up screen sharing for nas');
});

it.each(['WINDOWS', 'OTHER'] as const)('R-079: a %s device has no screen sharing button', (os) => {
  show(device({ os }));
  expect(setUp()).toBeUndefined();
});

it('R-079: with VNC on the VNC button shows and the set-up button does not', () => {
  show(device({ os: 'MACOS', vnc: { ...device().vnc, enabled: true } }));
  expect(setUp()).toBeUndefined();
  expect(document.body.textContent).toContain('VNC');
});

it('R-079: the button is hidden while no route gives the calls', () => {
  show(device({ os: 'MACOS' }), false);
  expect(setUp()).toBeUndefined();
});

it('R-079: a device that answers opens the VNC session from the button, then the page shows the updated device', async () => {
  const t = show(device({ os: 'LINUX' }));
  t.calls.test.mockResolvedValue(answer(true));
  setUp()?.click();
  await settle();
  flushSync();
  expect(t.onConnect).toHaveBeenCalledWith('VNC', expect.any(HTMLButtonElement));
  expect(t.onEnabled).toHaveBeenCalledWith(UPDATED);
  expect(t.calls.update).toHaveBeenCalledWith(device().id, { vnc: { enabled: true, port: 5900, username: null } });
});
