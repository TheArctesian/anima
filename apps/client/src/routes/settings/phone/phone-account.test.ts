// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { fakeHub, ok } from '$lib/access/access.fixture';
import { GATEWAY_KEPT } from '$lib/access/sign-out';
import PhoneAccount from './PhoneAccount.svelte';

// Sign out of the phone layout: in a phone browser behind the gateway it ends the session of the gateway too (HMD-059).

const mocks = vi.hoisted(() => ({
  hub: undefined as unknown,
  goToSignIn: vi.fn(),
  loadClientAt: vi.fn(),
  gateway: 'none' as 'none' | 'left' | 'kept',
}));

vi.mock('$lib/hub/context', () => ({ useHub: () => mocks.hub }));
vi.mock('$lib/platform/platform', async () => {
  const { BROWSER } = await import('$lib/platform/answers.fixture');
  return { capabilities: () => BROWSER };
});
vi.mock('$lib/hosted/gateway/here', () => ({ signOutOfGateway: () => Promise.resolve(mocks.gateway) }));
vi.mock('$lib/access/navigation', () => ({ goToSignIn: mocks.goToSignIn, loadClientAt: mocks.loadClientAt }));

let app: ReturnType<typeof mount> | undefined;

async function signOut(): Promise<HTMLElement> {
  mocks.hub = fakeHub({ Me: ok({ me: { username: 'owner', totpEnabled: false } }), Logout: ok({ logout: true }) }).hub;
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(PhoneAccount, { target });
  await new Promise((resolve) => setTimeout(resolve, 0));
  flushSync();
  target.querySelector('button')?.click();
  await new Promise((resolve) => setTimeout(resolve, 0));
  flushSync();
  return target;
}

beforeEach(() => {
  mocks.goToSignIn.mockReset();
  mocks.loadClientAt.mockReset();
  mocks.gateway = 'none';
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});

it('HMD-059: behind the gateway, Sign out on a phone signs out of the gateway and goes to the sign-in', async () => {
  mocks.gateway = 'left';
  await signOut();
  expect(mocks.goToSignIn).toHaveBeenCalledOnce();
  expect(mocks.loadClientAt).not.toHaveBeenCalled();
});

it('HMD-059: a gateway that does not confirm the sign-out is told on the phone too', async () => {
  mocks.gateway = 'kept';
  const target = await signOut();
  expect(target.querySelector('[role="alert"]')?.textContent).toBe(GATEWAY_KEPT);
  expect(mocks.goToSignIn).not.toHaveBeenCalled();
});

it('E-4: with no gateway, a phone browser loads the sign-in as a new page', async () => {
  await signOut();
  expect(mocks.loadClientAt).toHaveBeenCalledExactlyOnceWith('/sign-in');
});
