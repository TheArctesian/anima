// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import type { HubResult } from '$lib/hub/hub';
import { failure, fakeHub, ok } from './access.fixture';
import SignOutRow from './SignOutRow.svelte';
import { COOKIE_KEPT, GATEWAY_KEPT } from './sign-out';

// The Sign out row on both platforms (E-4): the desktop app clears its key store and moves within the page; a browser
// asks the hub to clear the cookie and then loads the sign-in page anew.

const mocks = vi.hoisted(() => ({
  hub: undefined as unknown,
  clearAccessToken: vi.fn(),
  goToSignIn: vi.fn(),
  loadClientAt: vi.fn(),
  desktop: true,
  gateway: 'none' as 'none' | 'left' | 'kept',
}));

vi.mock('$lib/hub/context', () => ({ useHub: () => mocks.hub }));
vi.mock('$lib/desktop/token', () => ({ clearAccessToken: mocks.clearAccessToken }));
vi.mock('$lib/platform/platform', async () => {
  const { BROWSER, DESKTOP_MAC } = await import('$lib/platform/answers.fixture');
  return { capabilities: () => (mocks.desktop ? DESKTOP_MAC : BROWSER) };
});
vi.mock('$lib/hosted/gateway/here', () => ({ signOutOfGateway: () => Promise.resolve(mocks.gateway) }));
vi.mock('./navigation', () => ({ goToSignIn: mocks.goToSignIn, loadClientAt: mocks.loadClientAt }));

let app: ReturnType<typeof mount> | undefined;
let sent: ReturnType<typeof fakeHub>['sent'];

const settled = async (): Promise<void> => {
  await new Promise((resolve) => setTimeout(resolve, 0));
  flushSync();
};

async function signOutWith(logout: HubResult<unknown>): Promise<HTMLElement> {
  const fake = fakeHub({ Me: ok({ me: { username: 'ann' } }), Logout: logout });
  mocks.hub = fake.hub;
  sent = fake.sent;
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(SignOutRow, { target });
  await settled();
  target.querySelector('button')?.click();
  await settled();
  return target;
}

beforeEach(() => {
  mocks.clearAccessToken.mockReset().mockResolvedValue(undefined);
  mocks.goToSignIn.mockReset();
  mocks.loadClientAt.mockReset();
  mocks.desktop = true;
  mocks.gateway = 'none';
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});

it('E-4: in a browser, Sign out sends Logout, touches no key store and loads /sign-in as a new page', async () => {
  mocks.desktop = false;
  await signOutWith(ok({ logout: true }));
  expect(sent.map((request) => request.operation)).toEqual(['Me', 'Logout']);
  expect(mocks.clearAccessToken).not.toHaveBeenCalled();
  expect(mocks.loadClientAt).toHaveBeenCalledExactlyOnceWith('/sign-in');
  expect(mocks.goToSignIn).not.toHaveBeenCalled();
});

it('CLI-003: in a browser, an unconfirmed logout stays on the page and says the browser may still be signed in', async () => {
  mocks.desktop = false;
  const target = await signOutWith(failure('unreachable', 'hub text'));
  expect(target.querySelector('[role="alert"]')?.textContent).toBe(COOKIE_KEPT);
  expect(mocks.loadClientAt).not.toHaveBeenCalled();
  expect(target.querySelector('button')?.disabled).toBe(false);
});

it('CLI-003: on desktop, Sign out clears the key store and goes to sign-in within the page', async () => {
  await signOutWith(ok({ logout: true }));
  expect(mocks.clearAccessToken).toHaveBeenCalledOnce();
  expect(mocks.goToSignIn).toHaveBeenCalledOnce();
  expect(mocks.loadClientAt).not.toHaveBeenCalled();
});

it('MOT-007: the alert is in the row from the start, empty, so a message only fills it', async () => {
  mocks.desktop = false;
  const fake = fakeHub({ Me: ok({ me: { username: 'ann' } }) });
  mocks.hub = fake.hub;
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(SignOutRow, { target });
  await settled();
  const idle = target.querySelector('[role="alert"]');
  expect(idle?.textContent).toBe('');
  target.querySelector('button')?.click();
  await settled();
  expect(target.querySelector('[role="alert"]')).toBe(idle);
  expect(idle?.textContent).toBe(COOKIE_KEPT);
});

it('E-4: in a browser the button stays off after a good sign-out, while the new page loads', async () => {
  mocks.desktop = false;
  const target = await signOutWith(ok({ logout: true }));
  expect(target.querySelector('button')?.disabled).toBe(true);
});

it('NAV-005: after an unconfirmed sign-out the keyboard focus is back on Sign out', async () => {
  mocks.desktop = false;
  const target = await signOutWith(failure('unreachable'));
  expect(document.activeElement).toBe(target.querySelector('button'));
});

it('HMD-062: behind the gateway, Sign out ends the session of the hub and of the gateway and loads no new page', async () => {
  mocks.desktop = false;
  mocks.gateway = 'left';
  await signOutWith(ok({ logout: true }));
  expect(sent.map((request) => request.operation)).toEqual(['Me', 'Logout']);
  // A new page would go to the gateway, which starts a sign-in at once.
  expect(mocks.loadClientAt).not.toHaveBeenCalled();
  expect(mocks.goToSignIn).toHaveBeenCalledOnce();
});

it('HMD-062: a gateway that does not confirm the sign-out is told, and the person can retry', async () => {
  mocks.desktop = false;
  mocks.gateway = 'kept';
  const target = await signOutWith(ok({ logout: true }));
  expect(target.querySelector('[role="alert"]')?.textContent).toBe(GATEWAY_KEPT);
  expect(mocks.goToSignIn).not.toHaveBeenCalled();
  expect(target.querySelector('button')?.disabled).toBe(false);
});
