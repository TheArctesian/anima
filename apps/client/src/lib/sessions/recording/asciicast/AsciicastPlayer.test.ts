// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { asciicastStream } from '$lib/fixtures/phase3';
import AsciicastPlayer from './AsciicastPlayer.svelte';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../../motion/engine.fixture')).fakeRun,
}));

let app: ReturnType<typeof mount> | undefined;
let saved: typeof window.matchMedia;

beforeEach(() => {
  saved = window.matchMedia;
  window.matchMedia = (() => ({ matches: false, addEventListener: () => undefined, removeEventListener: () => undefined, addListener: () => undefined, removeListener: () => undefined })) as unknown as typeof window.matchMedia;
});

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  window.matchMedia = saved;
  document.body.innerHTML = '';
});

function show(source: string, onfault = vi.fn()) {
  app = mount(AsciicastPlayer, { target: document.body, props: { source, complete: true, onfault } });
  flushSync();
  return onfault;
}

it('RA-041: the K6 asciicast recording opens with a Play button and a position slider', () => {
  const onfault = show(asciicastStream.text);
  expect(document.querySelector('button[aria-label="Play"]')).not.toBeNull();
  expect(document.querySelector('input[type="range"]')).not.toBeNull();
  expect(document.querySelector('.xterm')).not.toBeNull();
  expect(document.querySelector('[role="alert"]')).toBeNull();
  expect(onfault).not.toHaveBeenCalled();
});

it('RA-041: a recording that cannot be read shows the alert and reports the fault', () => {
  const onfault = show('not a recording\n');
  expect(document.querySelector('[role="alert"]')?.textContent).toBe('This recording cannot be read.');
  expect(document.querySelector('button')).toBeNull();
  expect(onfault).toHaveBeenCalledWith({ kind: 'header', line: 1 });
});
