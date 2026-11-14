// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import RunningRow from './RunningRow.svelte';

let animate: ReturnType<typeof vi.fn>;

beforeEach(() => {
  animate = vi.fn(() => ({ cancel: vi.fn() }));
  // jsdom has no Web Animations; a stand-in records what the row asks for.
  Object.defineProperty(HTMLElement.prototype, 'animate', { configurable: true, value: animate });
});

afterEach(() => {
  delete document.documentElement.dataset['motion'];
  Reflect.deleteProperty(HTMLElement.prototype, 'animate');
});

function show(live: boolean): () => void {
  const view = mount(RunningRow, { target: document.body, props: { href: '/devices', icon: 'carbon:screen', label: 'atlas', detail: 'RDP', live } });
  flushSync();
  return () => {
    unmount(view);
  };
}

it('UR-7 P2, MOT-011: a live row breathes while motion is allowed', () => {
  document.documentElement.dataset['motion'] = 'full';
  const close = show(true);
  expect(animate).toHaveBeenCalledTimes(1);
  close();
});

it('UR-7 P2, MOT-011: with reduced motion the live mark holds still', () => {
  document.documentElement.dataset['motion'] = 'reduce';
  const close = show(true);
  expect(document.querySelector('.dot')).not.toBeNull();
  expect(animate).not.toHaveBeenCalled();
  close();
});
