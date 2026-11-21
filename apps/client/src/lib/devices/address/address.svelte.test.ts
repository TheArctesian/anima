// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import CopyAddress from './CopyAddress.svelte';

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  vi.unstubAllGlobals();
  document.body.replaceChildren();
});

function show(writeText: (text: string) => Promise<void>): HTMLButtonElement {
  vi.stubGlobal('navigator', { clipboard: { writeText } });
  app = mount(CopyAddress, { target: document.body, props: { host: '10.40.0.14' } });
  flushSync();
  const button = document.querySelector('button');
  if (button === null) throw new Error('no copy button');
  return button;
}

it('MOB-015: Copy address puts the address on the clipboard and says so', async () => {
  const writeText = vi.fn(() => Promise.resolve());
  const button = show(writeText);
  expect(document.body.textContent).toContain('10.40.0.14');
  expect(button.getAttribute('aria-label')).toBe('Copy address');
  button.click();
  await vi.waitFor(() => { expect(button.getAttribute('aria-label')).toBe('Address copied'); });
  expect(writeText).toHaveBeenCalledWith('10.40.0.14');
});

it('MOB-015: a refused clipboard says the address was not copied', async () => {
  const button = show(() => Promise.reject(new Error('denied')));
  button.click();
  await vi.waitFor(() => { expect(button.getAttribute('aria-label')).toBe('Could not copy the address'); });
});
