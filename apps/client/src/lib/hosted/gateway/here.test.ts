// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { isHostedHub } from '../mark';

// The gateway as a browser page meets it. The network side has its own tests with a stand-in gateway
// (`$lib/hub/gateway`); here the two calls of it answer as the test says.

const mocks = vi.hoisted(() => ({
  desktop: false,
  /** Whether a gateway is in front, and whether it confirms a sign-out. */
  gateway: { inFront: true, confirms: true },
  asked: 0,
  signedOut: 0,
}));

vi.mock('$lib/hub/hub', async (original) => ({
  ...(await original<typeof import('$lib/hub/hub')>()),
  gatewayInFront: () => {
    mocks.asked += 1;
    return Promise.resolve(mocks.gateway.inFront);
  },
  gatewaySignOut: () => {
    mocks.signedOut += 1;
    return Promise.resolve(mocks.gateway.confirms);
  },
}));

vi.mock('$lib/platform/platform', async () => {
  const { BROWSER, DESKTOP_MAC } = await import('$lib/platform/answers.fixture');
  return { capabilities: () => (mocks.desktop ? DESKTOP_MAC : BROWSER) };
});

/** The module anew, as a page load gives it: it asks the gateway once for each load. */
async function page() {
  vi.resetModules();
  return import('./here');
}

const assign = vi.fn();

beforeEach(() => {
  mocks.desktop = false;
  mocks.gateway = { inFront: true, confirms: true };
  mocks.asked = 0;
  mocks.signedOut = 0;
  sessionStorage.clear();
  localStorage.clear();
  assign.mockReset();
  vi.stubGlobal('location', { origin: 'https://hub.example.net', assign });
});

afterEach(() => {
  vi.unstubAllGlobals();
});

it('HMD-059: a page behind the gateway asks once, and marks its hub as a hosted one', async () => {
  const here = await page();
  expect(await here.behindGateway()).toBe(true);
  expect(await here.behindGateway()).toBe(true);
  expect(mocks.asked).toBe(1);
  expect(isHostedHub(localStorage, 'https://hub.example.net')).toBe(true);
});

it('a page of a hub alone is not behind a gateway and gets no mark', async () => {
  mocks.gateway.inFront = false;
  const here = await page();
  expect(await here.behindGateway()).toBe(false);
  expect(isHostedHub(localStorage, 'https://hub.example.net')).toBe(false);
  expect(await here.signOutOfGateway()).toBe('none');
  expect(mocks.signedOut).toBe(0);
});

it('the desktop app never asks: it reaches a hosted hub through its link', async () => {
  mocks.desktop = true;
  const here = await page();
  expect(await here.behindGateway()).toBe(false);
  expect(mocks.asked).toBe(0);
});

it('HMD-062: the sign-out ends the session of the gateway, and the browser then waits for the person', async () => {
  const here = await page();
  expect(here.mayEnterByItself(1_000)).toBe(true);
  expect(await here.signOutOfGateway()).toBe('left');
  expect(mocks.signedOut).toBe(1);
  expect(here.leftGateway()).toBe(true);
  expect(here.mayEnterByItself(1_000)).toBe(false);
});

it('a gateway that does not confirm the sign-out is told as kept', async () => {
  mocks.gateway.confirms = false;
  const here = await page();
  expect(await here.signOutOfGateway()).toBe('kept');
  expect(here.leftGateway()).toBe(false);
});

it('HMD-063: the entry is a new page at the entry of the gateway, and the next one by itself waits a minute', async () => {
  const here = await page();
  sessionStorage.setItem('anima.gatewayLeft', '1');
  here.enterGateway(10_000);
  expect(assign).toHaveBeenCalledExactlyOnceWith('/_door/enter');
  expect(here.leftGateway()).toBe(false);
  expect(here.mayEnterByItself(20_000)).toBe(false);
  expect(here.mayEnterByItself(70_000)).toBe(true);
});
