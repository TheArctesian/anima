// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import Page from './+page.svelte';

// The sign-in page of a hosted hub in a browser (HMD-059, HMD-063): no form and no password.

const mocks = vi.hoisted(() => ({ left: false, mayEnter: true, enterGateway: vi.fn() }));

vi.mock('$lib/hosted/gateway/here', () => ({
  leftGateway: () => mocks.left,
  mayEnterByItself: () => mocks.mayEnter,
  enterGateway: mocks.enterGateway,
}));

let app: ReturnType<typeof mount> | undefined;

function shown(): HTMLElement {
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(Page, { target });
  flushSync();
  return target;
}

beforeEach(() => {
  mocks.left = false;
  mocks.mayEnter = true;
  mocks.enterGateway.mockReset();
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});

it('HMD-059: a browser that must sign in goes to the entry of the gateway by itself, and the page has no password field', () => {
  const target = shown();
  expect(mocks.enterGateway).toHaveBeenCalledOnce();
  expect(target.querySelector('input')).toBeNull();
  expect(target.querySelector('form')).toBeNull();
  expect(target.querySelector('h1')?.textContent).toBe('Signing in');
  expect(target.querySelector('button')?.disabled).toBe(true);
});

it('HMD-062: after a sign-out the page waits, and "Sign in" goes to the entry', () => {
  mocks.left = true;
  mocks.mayEnter = false;
  const target = shown();
  expect(mocks.enterGateway).not.toHaveBeenCalled();
  expect(target.querySelector('h1')?.textContent).toBe('Signed out');
  const button = target.querySelector('button');
  expect(button?.textContent).toBe('Sign in');
  button?.click();
  expect(mocks.enterGateway).toHaveBeenCalledOnce();
});

it('HMD-063: when an entry just failed, the page says so in plain words and offers "Try again", with no loop', () => {
  mocks.mayEnter = false;
  const target = shown();
  expect(mocks.enterGateway).not.toHaveBeenCalled();
  expect(target.querySelector('p')?.textContent).toBe('Your hub could not open a session for this browser.');
  expect(target.querySelector('button')?.textContent).toBe('Try again');
});
