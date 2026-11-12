// @vitest-environment jsdom
import { createRawSnippet, flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it } from 'vitest';
import Input from './Input.svelte';
import Select from './Select.svelte';

let app: ReturnType<typeof mount> | undefined;

afterEach(() => {
  if (app) void unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

it('UI-013: invalid sets aria-invalid, and a valid input sets none', () => {
  app = mount(Input, { target: document.body, props: { invalid: true } });
  flushSync();
  expect(document.querySelector('input')?.getAttribute('aria-invalid')).toBe('true');
  void unmount(app);
  document.body.replaceChildren();
  app = mount(Input, { target: document.body, props: {} });
  flushSync();
  expect(document.querySelector('input')?.hasAttribute('aria-invalid')).toBe(false);
});

it('UI-011: the mono option marks machine text; the number type marks tabular figures', () => {
  app = mount(Input, { target: document.body, props: { mono: true, type: 'number' } });
  flushSync();
  const input = document.querySelector('input');
  expect(input?.classList.contains('mono')).toBe(true);
  expect(input?.classList.contains('number')).toBe(true);
});

it('UI-013: a select keeps its options, is invalid on request, and its chevron is hidden from assistive technology', () => {
  const options = createRawSnippet(() => ({ render: () => '<optgroup label="Ports"><option value="a">A</option><option value="b">B</option></optgroup>' }));
  app = mount(Select, { target: document.body, props: { children: options, invalid: true, 'aria-label': 'Port' } });
  flushSync();
  const select = document.querySelector('select');
  expect(select?.options).toHaveLength(2);
  expect(select?.getAttribute('aria-invalid')).toBe('true');
  expect(document.querySelector('.chevron [aria-hidden="true"]')).not.toBeNull();
});
