// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { storedHubAddress } from '$lib/desktop/address';
import Page from './+page.svelte';

const mocks = vi.hoisted(() => ({ keyStore: true, loadClientAt: vi.fn() }));

vi.mock('$app/paths', () => ({ resolve: (path: string) => path }));
vi.mock('$lib/access/navigation', () => ({ loadClientAt: mocks.loadClientAt }));
vi.mock('$lib/platform/platform', () => ({ capabilities: () => ({ keyStore: mocks.keyStore }) }));

let app: ReturnType<typeof mount> | undefined;

function open(): HTMLElement {
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(Page, { target });
  flushSync();
  return target;
}

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
  localStorage.clear();
  mocks.loadClientAt.mockReset();
  mocks.keyStore = true;
});

it('FRN-001: first-run hub address is collected before credentials and a valid address opens sign-in', () => {
  const target = open();
  const address = target.querySelector<HTMLInputElement>('#hub-address');
  const button = target.querySelector<HTMLButtonElement>('button[type="submit"]');
  if (address === null || button === null) throw new Error('Missing address form');
  expect(address.value).toBe('');
  expect(target.querySelector('#username, #password')).toBeNull();
  expect(mocks.loadClientAt).not.toHaveBeenCalled();

  button.click();
  flushSync();
  expect(address.getAttribute('aria-invalid')).toBe('true');
  expect(mocks.loadClientAt).not.toHaveBeenCalled();

  address.value = 'https://hub.example.net';
  address.dispatchEvent(new Event('input', { bubbles: true }));
  flushSync();
  button.click();
  expect(storedHubAddress(localStorage).origin).toBe('https://hub.example.net');
  expect(mocks.loadClientAt).toHaveBeenCalledExactlyOnceWith('/sign-in');
});

it('CLI-002: a browser that opens the address page can only continue to the serving hub', () => {
  mocks.keyStore = false;
  const target = open();
  expect(target.querySelector('input')).toBeNull();
  expect(target.querySelector('a[href="/sign-in"]')?.textContent).toBe('Sign in to this hub');
});
