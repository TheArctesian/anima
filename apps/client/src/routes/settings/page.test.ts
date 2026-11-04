// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it } from 'vitest';
import Page from './+page.svelte';

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});

it('SET-002: the page shows only the title "Settings", with no other heading, button, input or link', () => {
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(Page, { target });
  flushSync();
  expect(target.textContent.trim()).toBe('Settings');
  expect(target.querySelector('h1')?.textContent).toBe('Settings');
  expect(target.querySelectorAll('h2, h3, h4, h5, h6, [role="heading"]')).toHaveLength(0);
  expect(target.querySelectorAll('a, button, input, select, textarea')).toHaveLength(0);
});
