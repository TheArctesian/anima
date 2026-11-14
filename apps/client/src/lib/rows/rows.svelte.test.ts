// @vitest-environment jsdom
import { createRawSnippet, mount, unmount } from 'svelte';
import { afterEach, expect, it } from 'vitest';
import Row from './Row.svelte';

const text = (html: string) => createRawSnippet(() => ({ render: () => html }));
let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

it('UR-7, D9: a row that opens a page is one link holding its cells; a row without a page is not a link', async () => {
  app = mount(Row, { target: document.body, props: { to: '/devices', edge: 'down', children: text('<span>backup-01</span>') } });
  const link = document.querySelector('a');
  expect(link?.getAttribute('href')).toMatch(/\/devices$/);
  expect(link?.textContent).toContain('backup-01');
  expect(document.querySelector('li')?.dataset['edge']).toBe('down');
  await unmount(app);
  app = mount(Row, { target: document.body, props: { children: text('<span>nas</span>') } });
  expect(document.querySelector('a')).toBeNull();
  expect(document.querySelector('li')?.textContent).toContain('nas');
});
