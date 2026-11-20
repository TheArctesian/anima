// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it } from 'vitest';
import { DEFAULT_QUERY } from '../query';
import Tally from './Tally.svelte';

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

function show(props: Record<string, unknown>): void {
  app = mount(Tally, { target: document.body, props: { query: DEFAULT_QUERY, onQueryChange: () => undefined, ...props } });
  flushSync();
}

it('before the first list the line says that it loads, and no count is read', () => {
  show({ tally: null });
  expect(document.querySelector('[role="status"]')?.textContent).toBe('Loading the devices');
  expect(document.querySelector('[role="group"]')).toBeNull();
  // The hidden count only gives the line its height.
  expect(document.querySelector('button')?.getAttribute('aria-hidden')).toBe('true');
});

it('when no list can come the line is empty', () => {
  show({ tally: null, waiting: false });
  expect(document.querySelector('[role="status"]')).toBeNull();
});

it('with a list the line is the counts', () => {
  show({ tally: { total: 2, parts: [{ status: 'UP', count: 2, words: 'up' }] } });
  expect([...document.querySelectorAll('[role="group"] button')].map((button) => button.textContent.replace(/\s+/g, ' ').trim())).toEqual(['2devices', '2up']);
});
