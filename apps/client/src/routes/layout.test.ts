import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { FONT_WAIT_MS } from '$lib/typeface/first-paint';
import { load } from './+layout';

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
