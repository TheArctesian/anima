// @vitest-environment jsdom
import { createRawSnippet, flushSync, mount, tick, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { runs } from '../../motion/engine.fixture';
import Disclosure from './Disclosure.svelte';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../motion/engine.fixture')).fakeRun,
}));

const content = createRawSnippet(() => ({ render: () => '<p>settings</p>' }));
let app: ReturnType<typeof mount> | undefined;

afterEach(() => {
  if (app !== undefined) void unmount(app);
  app = undefined;
  runs.length = 0;
  document.body.innerHTML = '';
});

async function settle(): Promise<void> {
  for (let pass = 0; pass < 3; pass += 1) {
    await tick();
    await Promise.resolve();
  }
  flushSync();
}

it('5.7.3 disclosure: opened from outside while it fades out, the content ends visible', async () => {
  const props = $state({ label: 'Advanced', children: content, preference: { reduced: false }, open: false });
  app = mount(Disclosure, { target: document.body, props });
  flushSync();
  const button = document.querySelector('button') as HTMLButtonElement;
  const region = document.querySelector('.region') as HTMLElement;
  button.click();
  await settle();
  button.click();
  await settle();
  const closing = runs.at(-1);

  props.open = true;
  flushSync();
  closing?.finish();
  await settle();

  expect(button.getAttribute('aria-expanded')).toBe('true');
  expect(region.hidden).toBe(false);
  expect(region.style.opacity).not.toBe('0');
  expect(region.style.top).toBe('');
});
