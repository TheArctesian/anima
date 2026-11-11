// @vitest-environment jsdom
import { createRawSnippet, flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { FONT_WAIT_MS } from '$lib/typeface/first-paint';
import { load } from './+layout';
import Layout from './+layout.svelte';

// A page no hub served is refused and starts no hub, so the layout mounts with only the router stood in (E-2).
vi.mock('$app/state', () => ({ page: { url: new URL('http://hub.example.org/devices') } }));
vi.mock('$app/navigation', () => ({ goto: vi.fn(() => Promise.resolve()) }));
vi.mock('$app/paths', () => ({ resolve: (path: string) => path, asset: (path: string) => path }));

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

/** Whether the promise has settled, without waiting for it. */
async function settled(promise: Promise<void>): Promise<boolean> {
  let done = false;
  void promise.then(() => {
    done = true;
  });
  await vi.advanceTimersByTimeAsync(0);
  return done;
}

it('MOT-007: the first render does not resolve before the fonts load', async () => {
  let loaded: (faces: FontFace[]) => void = () => undefined;
  const pending = new Promise<FontFace[]>((resolve) => {
    loaded = resolve;
  });
  vi.stubGlobal('localStorage', { getItem: () => null });
  vi.stubGlobal('document', { fonts: { load: () => pending } });

  const result = load();
  expect(await settled(result)).toBe(false);
  loaded([]);
  expect(await settled(result)).toBe(true);
});

it('MOT-007: the first render stops waiting for fonts after the cap', async () => {
  vi.stubGlobal('localStorage', { getItem: () => null });
  vi.stubGlobal('document', { fonts: { load: () => new Promise<FontFace[]>(() => undefined) } });

  const result = load();
  await vi.advanceTimersByTimeAsync(FONT_WAIT_MS - 1);
  expect(await settled(result)).toBe(false);
  await vi.advanceTimersByTimeAsync(1);
  expect(await settled(result)).toBe(true);
});

it('F-6, F-7, UI-005, D-28: the stored theme and motion choice are on <html> from the start', () => {
  vi.useRealTimers();
  localStorage.setItem('anima.theme', 'light');
  localStorage.setItem('anima.motion', 'reduce');
  const target = document.createElement('div');
  document.body.append(target);
  const app = mount(Layout, { target, props: { children: createRawSnippet(() => ({ render: () => '<p>the page</p>' })) } });
  flushSync();
  const root = document.documentElement;
  const started = { theme: root.dataset['theme'], motion: root.dataset['motion'] };
  void unmount(app);
  target.remove();
  localStorage.clear();
  delete root.dataset['theme'];
  delete root.dataset['motion'];
  expect(started).toEqual({ theme: 'light', motion: 'reduce' });
});
